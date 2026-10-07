# BuilderCraft Git setup

The source checkout tracks CADCraft as `upstream`. The active branch is `buildercraft/alpha-foundation`; the earlier `buildercraft/rhino-interface` branch preserves the unmodified baseline.

The delivery includes a full Git bundle. Restore its history with:

```sh
git clone --branch buildercraft/alpha-foundation BuilderCraft.bundle BuilderCraft-with-history
cd BuilderCraft-with-history
git remote remove origin
git remote add upstream https://github.com/storytold/cadcraft.git
```

After a user-owned GitHub fork/repository has been created, add its actual URL as `origin` and push the alpha branch. No user-owned hosted repository is connected yet. Do not push BuilderCraft changes to CADCraft's upstream repository.

```sh
git remote add origin YOUR_BUILDERCRAFT_REPOSITORY_URL
git push -u origin buildercraft/alpha-foundation
```

The alpha retains CADCraft's MIT/Apache notices and removes upstream trademark assets from the current source tree. Keep feature work on small branches and preserve a working mainline as the project evolves.
