#!/bin/sh
set -eu

data_dir="${DATA_DIR_PATH:-/app_data}"
music_dir="$data_dir/music"

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

mkdir -p "$data_dir" "$data_dir/covers" "$data_dir/.cache"
touch "$data_dir/mus.db"
chown "$runtime_uid:$runtime_gid" "$data_dir" "$data_dir/mus.db"
chown -R "$runtime_uid:$runtime_gid" "$data_dir/covers" "$data_dir/.cache"
chown "$runtime_uid:$runtime_gid" /usr/local/bin/yt-dlp

exec su-exec "$runtime_uid:$runtime_gid" "$@"
