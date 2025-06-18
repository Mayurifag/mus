#!/bin/bash
set -e

# Substitute environment variables in nginx.conf.template
envsubst '${SECRET_KEY}' < /app/docker/production/nginx.conf.template > /etc/nginx/nginx.conf

# Start supervisord
exec /usr/bin/supervisord -c /etc/supervisor/conf.d/supervisord.conf
