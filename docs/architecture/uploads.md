# Upload System Architecture

**Status:** Accepted for phase 8  
**Scope:** Upload metadata, local and S3-compatible storage, image processing, avatars, Markdown images, and attachments

## Design goals

- Keep object storage behind a provider-neutral Rust trait.
- Store authoritative metadata and lifecycle state in PostgreSQL.
- Validate file content, not only client-provided names and MIME headers.
- Generate server-owned object keys; never use an original filename as a path.
- Keep uploads private to their owner for metadata and deletion operations.
- Return stable public URLs that can later point at a CDN without schema changes.
- Leave multipart object upload and asynchronous cleanup as explicit extension points.

## Components

```text
multipart HTTP request
        |
        v
Upload Handler -> Upload Service -> image validation/processing
                         |                    |
                         v                    v
                 Upload Repository     StorageProvider
                         |              /             \
                     PostgreSQL   LocalStorage      S3Storage
```

The handler only parses multipart fields and authenticated identity. The service owns policy,
validation, key generation, compensation, and authorization. The repository owns SQL. Storage
implementations only put/delete objects and construct public URLs.

## Upload categories

| Category        | Accepted content     | Maximum size | Image processing                                     |
| --------------- | -------------------- | -----------: | ---------------------------------------------------- |
| `avatar`        | JPEG, PNG, WebP      |        5 MiB | resize to fit 512x512, compress, thumbnail 128x128   |
| `topic_image`   | JPEG, PNG, WebP, GIF |       10 MiB | resize to fit 2560x2560, compress, thumbnail 480x480 |
| `comment_image` | JPEG, PNG, WebP, GIF |        8 MiB | resize to fit 1920x1920, compress, thumbnail 480x480 |
| `attachment`    | see allowlist below  |       50 MiB | none                                                 |

Attachment allowlist (verified by magic bytes, never by filename or the
client-declared MIME type):

| Group     | Types                                                           |
| --------- | --------------------------------------------------------------- |
| Documents | PDF, plain text, RTF, DOC/DOCX, XLS/XLSX, PPT/PPTX, ODT/ODS/ODP |
| Archives  | ZIP, 7z, RAR, TAR, GZ, BZ2, XZ, ZSTD                            |
| Audio     | MP3, M4A, OGG, Opus, FLAC, WAV, AAC, MIDI                       |
| Video     | MP4, WebM, MKV, MOV, AVI, WMV, MPEG, M4V                        |

SVG and executable formats (PE/ELF/Mach-O, Wasm, Java classes, shell scripts,
HTML, XML, fonts, MSI, Debian packages, certificates) are rejected. Image MIME
is derived by decoding the bytes. Attachment MIME is checked by magic bytes
where possible and compared with the allowlist; text without a signature is
accepted only when it is UTF-8, contains no NUL bytes, and does not start like
an active-content document or shebang script.

## Serving security

Uploaded objects are never rendered inside the forum origin:

- Images are re-encoded server-side (except animated GIF, which keeps its
  bytes but is served as `image/gif`) and are the only content served inline.
- Every non-image object is stored under a server-owned extension and is
  served with `Content-Disposition: attachment` so the browser downloads it
  instead of rendering it (PDF with embedded scripts, XML, etc.).
- All `/storage` responses carry `X-Content-Type-Options: nosniff`; combined
  with the server-verified `Content-Type` this prevents content-type
  confusion attacks.
- The local storage provider applies these headers in `routes/mod.rs`.
  S3/R2 stores the same disposition as object metadata; the CDN or reverse
  proxy in front of `S3_PUBLIC_URL` must pass it through and add `nosniff`.
- Original filenames are never used as paths or served filenames; objects are
  stored as `{uuid}.{verified_extension}` and the original name is display
  metadata only.
- Decompression bombs are bounded: images are dimension-capped (40 MP) before
  decoding, and archives are never extracted server-side, so a "zip bomb"
  costs at most the size cap in disk and bandwidth.
- Upload creation is rate limited per user (30 per 10 minutes) and gated by
  the moderation reputation system.

## Object key policy

Keys are generated exclusively by the service:

```text
{category}/{yyyy}/{mm}/{user_uuid}/{upload_uuid}.{verified_extension}
{category}/{yyyy}/{mm}/{user_uuid}/{upload_uuid}-thumb.{verified_extension}
```

All segments except the fixed category and date are generated server-side. Local storage joins the
key below a configured root and verifies that every key component is a normal path component. This
prevents absolute paths, `..`, and platform-specific traversal.

The original filename is retained only as display metadata after removing control characters,
directory components, and excessive length. It is never used to access storage.

## Lifecycle and consistency

Upload states are `pending`, `ready`, `deleting`, `deleted`, and `failed`.

1. Validate request and create a `pending` database row.
2. Put the main object and optional thumbnail.
3. Mark the row `ready` with its public URLs and image dimensions.
4. If storage fails, mark the row `failed`; if a later database write fails, delete uploaded objects
   as compensation.
5. Delete changes `ready` to `deleting`, removes objects, then changes it to `deleted`.
6. A storage deletion failure restores `ready`, so the operation can be retried.

Only `ready` records are returned by normal read/list endpoints. Database records are soft-deleted
to preserve auditability and to support cleanup reconciliation.

## API

- `POST /uploads`: authenticated multipart upload with `file` and `category` fields.
- `GET /uploads/{id}`: authenticated owner metadata lookup.
- `DELETE /uploads/{id}`: authenticated owner deletion.
- `GET /users/{id}/uploads`: authenticated; owner-only paginated list.
- `POST /users/profile/avatar`: authenticated avatar upload and atomic profile association.
- `DELETE /users/profile/avatar`: authenticated avatar association removal and object deletion.

The API process accepts at most the global attachment limit plus multipart overhead. Category limits
are enforced while reading the multipart field and again by the service.

## Storage configuration

`STORAGE_PROVIDER` is `local` or `s3`. Local development uses `STORAGE_LOCAL_ROOT` and
`STORAGE_PUBLIC_URL`. S3-compatible providers use endpoint, region, bucket, access key, secret key,
path-style behavior, and a public base URL. R2 and MinIO use `S3Storage` with different endpoint and
path-style settings; no provider-specific business code is required.

Credentials are process environment variables and are never persisted or returned by the API.

## Future extensions

- Multipart uploads add a session table and provider methods without changing completed upload rows.
- CDN adoption changes only `STORAGE_PUBLIC_URL` / `S3_PUBLIC_URL`.
- WebP conversion can be enabled in the image processor while preserving the same API contract.
- A scheduled reconciliation job can delete expired `pending`/`failed` objects and retry `deleting`
  rows.
- Content scanning can be inserted after validation and before the `ready` transition.
