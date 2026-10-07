#pragma once
#include "CoreMinimal.h"
#include "GameFramework/Actor.h"
#include "BuilderCraftSceneFeed.generated.h"
class UProceduralMeshComponent;
class UMaterialInterface;

UCLASS()
class BUILDERCRAFTBRIDGE_API ABuilderCraftSceneFeed : public AActor {
    GENERATED_BODY()
public:
    ABuilderCraftSceneFeed();
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="BuilderCraft") FString SnapshotPath;
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="BuilderCraft") FString ExpectedProjectId;
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="BuilderCraft", meta=(ClampMin="0.25")) float PollSeconds=0.5f;
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="BuilderCraft") bool bCreateCollision=false;
    UPROPERTY(EditAnywhere, BlueprintReadWrite, Category="BuilderCraft") TObjectPtr<UMaterialInterface> MassingMaterial;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category="BuilderCraft") FString ProductionJson;
    UPROPERTY(VisibleAnywhere, BlueprintReadOnly, Category="BuilderCraft") FString LastError;
    UFUNCTION(CallInEditor, BlueprintCallable, Category="BuilderCraft") void Refresh();
    virtual void Tick(float DeltaSeconds) override;
    virtual bool ShouldTickIfViewportsOnly() const override { return true; }
private:
    UPROPERTY(Transient) TMap<FString,TObjectPtr<UProceduralMeshComponent>> Parts;
    TMap<FString,FString> GeometryKeys;
    FString BoundProject;
    uint64 AppliedSequence=0;
    float Elapsed=0;
    FDateTime SeenStamp=FDateTime::MinValue();
    int64 SeenSize=-1;
    bool ReadAndApply();
};
