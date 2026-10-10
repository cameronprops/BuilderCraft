# Reference artwork, image planes and calibration

**Status:** Reference library requirements exist in [reference library roadmap](../reference-library-roadmap.md); a full image-plane/reference editor is an **alpha gate**. Rich tagging, boards, and multisource alignment can follow in beta.

## Alpha workflow

1. Open **Add Reference** and choose an authorized local PNG or JPEG (WebP when decoder verified). Do not run scripts, activate links or fetch external content without explicit user action.
2. Place as **screen overlay**, **2D image underlay**, or **3D image plane** with translation, rotation and sizing controls. Use a dedicated Reference layer/category.
3. Calibrate using two picked points and a known length, with explicit units. Verify scale stays correct after save/reopen.
4. Adjust opacity and crop, flip/front/back, layer, lock and visibility. Locked image planes should not steal normal modeling picks.
5. Choose **Embed** (small image, size-limited) or **Link** (relative path plus content hash, missing-link banner and relink action).
6. Record creator/source URL, license/use permission, source date, version and notes. Never bundle copyrighted design references with distributable example projects without permission.

## File and scene data contract

`ReferenceAsset { id, type=image, original_uri, relative_path?, checksum, bytes?, media_type, width_px, height_px, source, license, attribution, revision }`

`ReferencePlacement { id, asset_id, scene_id, target_layer?, coordinate_system, transform, width_in_document_units, opacity, crop, visible, locked, draw_order }`

Asset records and placement instances are separate, allowing one image to appear in multiple ortho views or scene shots without duplicate pixels. File relinking never changes physical placement. Reference images have selectable handles only in Edit Reference mode; default object selection filters may exclude them.

**Acceptance:** import test chart image, set its known measurement to 1 m, move to a chosen plane, lock it, draw in front of it, save/reopen offline and relink after moving the file. Verify controlled missing/over-budget/corrupt image errors, selection exclusion and that no opaque reference blocks geometry picking.

**Beta:** perspective camera match, multisheet drawing registration, reference mood boards, richer image/PDF/SVG formats, annotation and OCR/search with user control.
