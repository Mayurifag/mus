#!/bin/bash
set -e

cp /app/docker/production/nginx.conf.template /etc/nginx/nginx.conf

# Start supervisord
exec /usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf
