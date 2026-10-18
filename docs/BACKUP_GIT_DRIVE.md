# BuilderCraft — GitHub and Google Drive backups

GitHub is the source of truth. The GitHub Actions backup workflow makes a portable full Git bundle, exact source TAR archive, commit SHA and SHA-256 hashes. It verifies the bundle before publishing an Actions artifact (30-day retention).

## Verified Drive checkpoint

A complete snapshot of commit `a399caa9381af4966008dc562f46e1aebc904d82` was uploaded to `ChatGPT/BuilderCraft Backups/`. Drive file ID: `1kT4oeB_rpzy4TeQrvoU5b5PcUNN8Q4oP`. Its ZIP integrity and embedded commit ID have been verified.

## Enable automatic mirroring (not yet configured)

GitHub Actions cannot use the OAuth permissions from the connected ChatGPT Drive account. Configure a dedicated Google Drive API service account with write access to the backup folder.

1. Enable Google Drive API and create a dedicated Google Cloud service account with tightly limited backup-folder access.
2. Give that service account Editor access to the designated backup folder.
3. Add GitHub Actions secret `BUILDERCRAFT_DRIVE_SA_JSON` containing the service account JSON credential. Do not commit it.
4. Add GitHub Actions variable `BUILDERCRAFT_DRIVE_FOLDER_ID` with the destination folder ID `1AbTDQScgtm-4wXBpWYph1m1o62HhrkmG`.
5. Trigger the workflow, verify the Drive mirror step succeeds, and check the uploaded files. Missing credentials cause the Drive step to skip, *not* to upload.

The upload creates a distinct destination for each branch and commit. It uses a temporary credentials file removed when the step exits, and rclone checks the uploaded files against local source checksums.

## Recovery

Unpack the ZIP. Run `sha256sum -c SHA256SUMS.txt` alongside the two archived payloads. To recover full Git history, run:

```bash
git clone BuilderCraft-all-refs.bundle BuilderCraft-recovered
cd BuilderCraft-recovered
git branch -a
```

Alternatively extract the source archive to recover the exact tracked files at the snapshot commit.

Only tracked Git content and fetched refs are included. Ignored binaries, local-only files, GitHub secrets, discussions, issues, and PR review history are not included.