using UnrealBuildTool;
public class BuilderCraftBridge : ModuleRules {
    public BuilderCraftBridge(ReadOnlyTargetRules Target) : base(Target) {
        PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
        PublicDependencyModuleNames.AddRange(new string[] {"Core", "CoreUObject", "Engine", "ProceduralMeshComponent"});
        PrivateDependencyModuleNames.AddRange(new string[] {"Json", "JsonUtilities"});
    }
}
