#!/bin/bash
# Uploads files to Cloudflare R2, using the keys in .env.local. Run from the project folder:
#   bash packaging/release/upload-r2.sh 3.1.0                    the downloads of one version
#                                                                -> scientific-calculator/3.1.0/<file>
#   bash packaging/release/upload-r2.sh docs/site/images site    every file in a folder
#                                                                -> scientific-calculator/site/<file>
set -euo pipefail
cd "$(dirname "$0")/../.."
if [ -d "${1:-}" ]; then
  SRC_DIR="$1"; VERSION="${2:?give the folder name on R2, e.g. site}"
else
  SRC_DIR=""; VERSION="${1:-$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/')}"
fi

[ -f .env.local ] || { echo ".env.local not found in $(pwd)"; exit 1; }
# Value of a setting in .env.local: everything after the first "=", spaces trimmed.
val() { grep -iE "^[[:space:]]*$1[[:space:]]*=" .env.local | head -1 | sed -E 's/^[^=]*=[[:space:]]*//; s/[[:space:]]*$//'; }
ACCOUNT=$(val "Account_ID")
BUCKET=$(val "Bucket")
KEY_ID=$(val "Access_Key_ID")
SECRET=$(val "Secret Access Key")
[ -n "$ACCOUNT" ] && [ -n "$BUCKET" ] && [ -n "$KEY_ID" ] && [ -n "$SECRET" ] || { echo "Account_ID, Bucket, Access_Key_ID or Secret Access Key missing in .env.local"; exit 1; }
ENDPOINT="https://${ACCOUNT}.r2.cloudflarestorage.com/${BUCKET}"

shopt -s nullglob
if [ -n "$SRC_DIR" ]; then files=("$SRC_DIR"/*); else files=(downloads/*"${VERSION}"*); fi
[ ${#files[@]} -gt 0 ] || { echo "No files to upload for ${VERSION}"; exit 1; }

for f in "${files[@]}"; do
  [ -f "$f" ] || continue
  name=$(basename "$f")
  [ "$name" = ".DS_Store" ] && continue
  case "$name" in
    *.zip) type="application/zip" ;;
    *.msi) type="application/x-msi" ;;
    *.exe) type="application/vnd.microsoft.portable-executable" ;;
    *.deb) type="application/vnd.debian.binary-package" ;;
    *.tar.gz) type="application/gzip" ;;
    *.png) type="image/png" ;;
    *.webp) type="image/webp" ;;
    *.gif) type="image/gif" ;;
    *.jpg|*.jpeg) type="image/jpeg" ;;
    *.svg) type="image/svg+xml" ;;
    *.mp4) type="video/mp4" ;;
    *.webm) type="video/webm" ;;
    *) type="application/octet-stream" ;;
  esac
  # Installers download as files; images display in the page and are cached for a day.
  case "$type" in
    image/*|video/*) extra=(-H "Cache-Control: public, max-age=86400") ;;
    *) extra=(-H "Content-Disposition: attachment; filename=\"${name}\"") ;;
  esac
  hash=$(shasum -a 256 "$f" | cut -d' ' -f1)
  printf "Uploading %-55s " "$name"
  code=$(curl -sS -o /tmp/r2-upload.out -w "%{http_code}" --aws-sigv4 "aws:amz:auto:s3" --user "${KEY_ID}:${SECRET}" \
    -H "x-amz-content-sha256: ${hash}" -H "Content-Type: ${type}" \
    "${extra[@]}" \
    -T "$f" "${ENDPOINT}/scientific-calculator/${VERSION}/${name}")
  if [ "$code" = "200" ]; then echo "ok"; else echo "failed (HTTP $code)"; cat /tmp/r2-upload.out; echo; fi
done
echo
echo "Uploaded to ${BUCKET}/scientific-calculator/${VERSION}/"
echo "If the bucket has public access (R2 > ${BUCKET} > Settings > Public access), the files are at"
echo "  <public URL>/scientific-calculator/${VERSION}/<file name>"
