#!/bin/sh
set -eu

data_dir="${DATA_DIR_PATH:-/app_data}"
music_dir="$data_dir/music"
cache_dir="$data_dir/.cache"

if [ -d "$music_dir" ]; then
  runtime_uid="$(stat -c '%u' "$music_dir")"
  runtime_gid="$(stat -c '%g' "$music_dir")"
else
  runtime_uid="1000"
  runtime_gid="1000"
fi

if [ "$runtime_uid" = "0" ]; then
  exec "$@"
fi

group_name="$(awk -F: -v gid="$runtime_gid" '$3 == gid { print $1; exit }' /etc/group)"
if [ -z "$group_name" ]; then
  group_name="appgroup"
  addgroup -g "$runtime_gid" "$group_name"
fi

user_name="$(awk -F: -v uid="$runtime_uid" '$3 == uid { print $1; exit }' /etc/passwd)"
if [ -z "$user_name" ]; then
  user_name="appuser"
  adduser -D -H -u "$runtime_uid" -G "$group_name" "$user_name"
fi

export HOME="$data_dir/.cache"

mkdir -p "$data_dir" "$cache_dir"
# Rust owns migration, including checkpointing the legacy database's WAL.
for legacy_path in "$data_dir/mus.db" "$data_dir/mus.db-wal" "$data_dir/mus.db-shm" "$data_dir/covers"; do
  if [ -e "$legacy_path" ]; then
    chown -R "$runtime_uid:$runtime_gid" "$legacy_path"
  fi
done
chown "$runtime_uid:$runtime_gid" "$data_dir"
chown -R "$runtime_uid:$runtime_gid" "$cache_dir"
chown "$runtime_uid:$runtime_gid" /usr/local/bin /usr/local/bin/yt-dlp

exec su-exec "$runtime_uid:$runtime_gid" "$@"
