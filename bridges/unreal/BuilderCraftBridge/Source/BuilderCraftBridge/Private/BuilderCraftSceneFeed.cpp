// Original adapter code. No engine implementation source is incorporated.
#include "BuilderCraftSceneFeed.h"
#include "ProceduralMeshComponent.h"
#include "Components/SceneComponent.h"
#include "HAL/FileManager.h"
#include "Misc/FileHelper.h"
#include "Misc/LexFromString.h"
#include "Serialization/Archive.h"
#include "Materials/MaterialInterface.h"
#include "Serialization/JsonReader.h"
#include "Serialization/JsonSerializer.h"
#include "Serialization/JsonWriter.h"

namespace {
struct FPart {
    FString Id,Name,Parent,Key;
    bool Visible=true;
    TArray<FVector> Points,Normals;
    TArray<int32> Faces;
};
bool Identity(const FString& Id) {
    if(Id.Len()!=32) return false;
    bool Nonzero=false;
    for(TCHAR C:Id) {if(!FChar::IsHexDigit(C)) return false; Nonzero|=(C!=TEXT('0'));}
    return Nonzero;
}
bool Number(const TSharedPtr<FJsonValue>& V,double& Out) {
    return V.IsValid() && V->TryGetNumber(Out) && FMath::IsFinite(Out);
}
}
ABuilderCraftSceneFeed::ABuilderCraftSceneFeed() {
    PrimaryActorTick.bCanEverTick=true;
    SetRootComponent(CreateDefaultSubobject<USceneComponent>(TEXT("FeedRoot")));
}
void ABuilderCraftSceneFeed::Refresh() {SeenStamp=FDateTime::MinValue();SeenSize=-1;ReadAndApply();}
void ABuilderCraftSceneFeed::Tick(float DeltaSeconds) {
    Super::Tick(DeltaSeconds);
    Elapsed+=DeltaSeconds;
    if(Elapsed>=FMath::Max(0.25f,PollSeconds)) {Elapsed=0;ReadAndApply();}
}
bool ABuilderCraftSceneFeed::ReadAndApply() {
    auto Fail=[this](const TCHAR* Message) {LastError=Message; return false;};
    if(SnapshotPath.IsEmpty()) return false;
    const int64 Size=IFileManager::Get().FileSize(*SnapshotPath);
    const FDateTime Stamp=IFileManager::Get().GetTimeStamp(*SnapshotPath);
    if(Size==SeenSize && Stamp==SeenStamp) return true;
    if(Size<0 || Size>32*1024*1024) return Fail(TEXT("Snapshot absent or exceeds 32 MiB"));
    TUniquePtr<FArchive> Reader(IFileManager::Get().CreateFileReader(*SnapshotPath));
    if(!Reader || Reader->TotalSize()!=Size) return Fail(TEXT("Snapshot changed during open; retry"));
    TArray<uint8> Buffer;Buffer.SetNumUninitialized(static_cast<int32>(Size));
    Reader->Serialize(Buffer.GetData(),Size);
    if(Reader->IsError()) return Fail(TEXT("Snapshot read failed"));
    Reader.Reset();
    FString Text;FFileHelper::BufferToString(Text,Buffer.GetData(),Buffer.Num());
    int32 Depth=0;bool Quoted=false,Escaped=false;
    for(TCHAR C:Text) {
        if(Quoted) {if(Escaped) Escaped=false;else if(C==TEXT('\\')) Escaped=true;else if(C==TEXT('"')) Quoted=false;}
        else if(C==TEXT('"')) Quoted=true;
        else if(C==TEXT('{') || C==TEXT('[')) {if(++Depth>128) return Fail(TEXT("JSON nesting limit"));}
        else if(C==TEXT('}') || C==TEXT(']')) {if(--Depth<0) return Fail(TEXT("Invalid JSON nesting"));}
    }
    if(Depth!=0 || Quoted) return Fail(TEXT("Incomplete JSON"));
    TSharedPtr<FJsonObject> Json;
    if(!FJsonSerializer::Deserialize(TJsonReaderFactory<>::Create(Text),Json) || !Json) return Fail(TEXT("Invalid snapshot JSON"));
    double Version=0;FString Project,Sequence;
    if(!Json->TryGetNumberField(TEXT("protocol_version"),Version) || Version!=1 || !Json->TryGetStringField(TEXT("project_id"),Project)
       || !Identity(Project) || !Json->TryGetStringField(TEXT("sequence"),Sequence)) return Fail(TEXT("Unsupported snapshot protocol"));
    uint64 Next=0;
    if(Sequence.IsEmpty() || Sequence.Len()>20) return Fail(TEXT("Invalid sequence"));
    for(TCHAR C:Sequence) if(!FChar::IsDigit(C)) return Fail(TEXT("Invalid sequence"));
    if(!LexTryParseString(Next,*Sequence) || Next==0) return Fail(TEXT("Invalid sequence"));
    if((!ExpectedProjectId.IsEmpty() && Project!=ExpectedProjectId) || (!BoundProject.IsEmpty() && Project!=BoundProject)) return Fail(TEXT("Project mismatch"));
    if(Next<=AppliedSequence) {SeenStamp=Stamp;SeenSize=Size;return true;}
    const TSharedPtr<FJsonObject>* Frame=nullptr;FString Unit,Axes;
    if(!Json->TryGetObjectField(TEXT("frame"),Frame) || !Frame || !(*Frame)->TryGetStringField(TEXT("unit"),Unit)
       || !(*Frame)->TryGetStringField(TEXT("axes"),Axes) || Unit!=TEXT("Metre") || Axes!=TEXT("LeftHandedZUp")) return Fail(TEXT("Unsupported coordinate frame"));
    const TArray<TSharedPtr<FJsonValue>>* Objects=nullptr;
    if(!Json->TryGetArrayField(TEXT("objects"),Objects) || Objects->Num()>256) return Fail(TEXT("Object count invalid"));
    TArray<FPart> Incoming;TSet<FString> Ids;int32 TotalPoints=0,TotalFaces=0;
    for(const auto& Value:*Objects) {
        const TSharedPtr<FJsonObject>* Object=nullptr;
        if(!Value.IsValid() || !Value->TryGetObject(Object) || !Object) return Fail(TEXT("Invalid object"));
        FPart Part;bool Polyline=false;
        if(!(*Object)->TryGetStringField(TEXT("id"),Part.Id) || !Identity(Part.Id) || Ids.Contains(Part.Id)
           || !(*Object)->TryGetStringField(TEXT("name"),Part.Name) || Part.Name.Len()>256
           || !(*Object)->TryGetStringField(TEXT("geometry_key"),Part.Key) || Part.Key.Len()>64
           || !(*Object)->TryGetBoolField(TEXT("visible"),Part.Visible) || !(*Object)->TryGetBoolField(TEXT("polyline"),Polyline)) return Fail(TEXT("Invalid object metadata"));
        (*Object)->TryGetStringField(TEXT("parent"),Part.Parent);
        if(!Part.Parent.IsEmpty() && !Identity(Part.Parent)) return Fail(TEXT("Invalid parent"));
        Ids.Add(Part.Id);
        const TArray<TSharedPtr<FJsonValue>>* Points=nullptr;const TArray<TSharedPtr<FJsonValue>>* Faces=nullptr;
        if(!(*Object)->TryGetArrayField(TEXT("positions"),Points) || !(*Object)->TryGetArrayField(TEXT("triangles"),Faces)) return Fail(TEXT("Missing geometry arrays"));
        TotalPoints+=Points->Num();TotalFaces+=Faces->Num();
        if(TotalPoints>50000 || TotalFaces>100000) return Fail(TEXT("Aggregate geometry count limit"));
        for(const auto& Point:*Points) {
            const TArray<TSharedPtr<FJsonValue>>* P=nullptr;double X,Y,Z;
            if(!Point.IsValid() || !Point->TryGetArray(P) || P->Num()!=3 || !Number((*P)[0],X) || !Number((*P)[1],Y) || !Number((*P)[2],Z)
               || FMath::Abs(X)>1e7 || FMath::Abs(Y)>1e7 || FMath::Abs(Z)>1e7) return Fail(TEXT("Invalid position"));
            Part.Points.Add(FVector(X*100.0,Y*100.0,Z*100.0)); // metres to Unreal centimetres
        }
        for(const auto& Face:*Faces) {
            const TArray<TSharedPtr<FJsonValue>>* F=nullptr;
            if(!Face.IsValid() || !Face->TryGetArray(F) || F->Num()!=3) return Fail(TEXT("Invalid triangle"));
            for(const auto& Index:*F) {
                double I;
                if(!Number(Index,I) || I<0 || I>=Part.Points.Num() || FMath::FloorToDouble(I)!=I) return Fail(TEXT("Triangle index out of range"));
                Part.Faces.Add(static_cast<int32>(I));
            }
        }
        if(Polyline) {Part.Points.Reset();Part.Faces.Reset();} // portable GLB includes curves; first live adapter visualizes surfaces.
        Part.Normals.Init(FVector::ZeroVector,Part.Points.Num());
        for(int32 I=0;I<Part.Faces.Num();I+=3) {
            const int32 A=Part.Faces[I],B=Part.Faces[I+1],C=Part.Faces[I+2];
            const FVector N=FVector::CrossProduct(Part.Points[B]-Part.Points[A],Part.Points[C]-Part.Points[A]);
            Part.Normals[A]+=N;Part.Normals[B]+=N;Part.Normals[C]+=N;
        }
        for(auto& N:Part.Normals) N=N.GetSafeNormal();
        Incoming.Add(MoveTemp(Part));
    }
    TMap<FString,FString> Parents;
    for(const auto& P:Incoming) {if(!P.Parent.IsEmpty() && !Ids.Contains(P.Parent)) return Fail(TEXT("Missing parent"));Parents.Add(P.Id,P.Parent);}
    for(const auto& P:Incoming) {
        TSet<FString> Seen;FString Current=P.Id;
        while(!Current.IsEmpty()) {
            if(Seen.Contains(Current) || Seen.Num()>=64) return Fail(TEXT("Cyclic or deep hierarchy"));
            Seen.Add(Current);Current=Parents.FindRef(Current);
        }
    }
    // Validate everything above before changing any existing scene component.
    TArray<FString> Created;
    for(const auto& P:Incoming) {
        if(Parts.Contains(P.Id) && !IsValid(Parts.FindRef(P.Id).Get())) {Parts.Remove(P.Id);GeometryKeys.Remove(P.Id);}
    }
    for(const auto& P:Incoming) if(!Parts.Contains(P.Id)) {
        auto* Mesh=NewObject<UProceduralMeshComponent>(this);
        if(!Mesh) {for(const auto& Id:Created){Parts[Id]->DestroyComponent();Parts.Remove(Id);}return Fail(TEXT("Component creation failed"));}
        Mesh->SetupAttachment(GetRootComponent());Mesh->bUseAsyncCooking=true;AddInstanceComponent(Mesh);Mesh->RegisterComponent();
        Parts.Add(P.Id,Mesh);Created.Add(P.Id);
    }
    for(const auto& P:Incoming) {
        auto* Mesh=Parts.FindRef(P.Id).Get();
        Mesh->AttachToComponent(P.Parent.IsEmpty()?GetRootComponent():Parts.FindRef(P.Parent).Get(),FAttachmentTransformRules::KeepRelativeTransform);
        Mesh->ComponentTags={FName(*P.Id),FName(*P.Name)};
        if(GeometryKeys.FindRef(P.Id)!=P.Key || Created.Contains(P.Id)) {
            Mesh->ClearAllMeshSections();
            if(!P.Faces.IsEmpty()) Mesh->CreateMeshSection(0,P.Points,P.Faces,P.Normals,TArray<FVector2D>(),TArray<FColor>(),TArray<FProcMeshTangent>(),bCreateCollision);
            GeometryKeys.Add(P.Id,P.Key);
        }
        if(MassingMaterial) Mesh->SetMaterial(0,MassingMaterial);
        Mesh->SetVisibility(P.Visible,false);
    }
    TArray<FString> Removed;
    for(const auto& Entry:Parts) if(!Ids.Contains(Entry.Key)) Removed.Add(Entry.Key);
    for(const auto& Id:Removed){Parts[Id]->DestroyComponent();Parts.Remove(Id);GeometryKeys.Remove(Id);}
    const TSharedPtr<FJsonObject>* Production=nullptr;
    if(Json->TryGetObjectField(TEXT("production"),Production)) {
        ProductionJson.Reset();FJsonSerializer::Serialize((*Production).ToSharedRef(),TJsonWriterFactory<>::Create(&ProductionJson));
    }
    BoundProject=Project;AppliedSequence=Next;SeenStamp=Stamp;SeenSize=Size;LastError.Reset();return true;
}
