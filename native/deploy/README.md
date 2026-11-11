# Deploying mmm-native on the Pi

The binary lives in `/opt/mmm-native/` (owned by `max`), runs as the systemd unit
`mmm-native.service` as user `max`, and reads the web app's config at
`/opt/stacks/mirror/data/config.json`. Token files live beside the config:
`google-tokens.json` (shared format with the web app) and `spotify-tokens.json`.

## One-time install

From the dev machine, in the repo root:

    ssh -t pi 'sudo install -d -o max -g max -m 755 /opt/mmm-native'
    scp native/deploy/mmm-native.service pi:/tmp/mmm-native.service
    ssh -t pi 'sudo install -m 644 /tmp/mmm-native.service /etc/systemd/system/ && sudo systemctl daemon-reload'
    ssh pi 'systemctl get-default'          # must print graphical.target (the unit is WantedBy it)

Stop the Chromium kiosk and the web app. Both stay installed, so rolling back is easy:

    ssh -t pi 'sudo systemctl disable --now dashboard-kiosk.service dashboard-monitor.service kiosk-page-watchdog.service dashboard-kiosk-nightly.timer'
    ssh pi 'cd /opt/stacks/mirror && docker compose down'

Let `max` write the token files. The web container wrote them as root, and this app
saves refreshed tokens with an atomic rename, so it needs the data directory as well:

    ssh pi 'ls -ld /opt/stacks/mirror/data; ls -l /opt/stacks/mirror/data'
    ssh -t pi 'sudo chown max:max /opt/stacks/mirror/data /opt/stacks/mirror/data/google-tokens.json && chmod 600 /opt/stacks/mirror/data/google-tokens.json'

Build, copy and start, then enable it at boot:

    native/deploy/deploy.sh
    ssh -t pi 'sudo systemctl enable mmm-native'

Re-authorise Spotify (both refresh tokens are revoked). Run this on the Pi, once per
account, and follow the prompts. The name must match `name` in
`components.spotify.accounts`:

    ssh -t pi '/opt/mmm-native/mmm-native auth spotify --name "<account name>" --config /opt/stacks/mirror/data/config.json'
    ssh -t pi 'sudo systemctl restart mmm-native'

Google works the same way: `auth google`. A running dashboard picks up new Google tokens
without a restart.

How `auth` works:
- It prints a URL. Open it on a phone or laptop and approve.
- The browser is then sent to the redirect URI, and that page fails to load. This is
  expected.
- Paste the whole address from the browser's address bar back into the terminal.

The redirect URI must be one registered for the OAuth client. The defaults are:
- Google: `http://localhost:4000/api/google/callback`
- Spotify: `http://127.0.0.1:4000/api/spotify/callback`. Spotify no longer accepts
  `localhost`; see https://developer.spotify.com/documentation/web-api/concepts/redirect_uri.

If one of these is not registered, add it in the provider's console, or pass the
registered one with `--redirect-uri`.

## Deploy a new build

    native/deploy/deploy.sh                   # MMM_PI=user@host to override the ssh host

The script does the following:
- cross-builds;
- checks that the binary links only libc/libm;
- copies it as `/opt/mmm-native/.mmm-native.new.<pid>`;
- hard-links the current binary to `mmm-native.prev`;
- renames the new one into place;
- restarts the unit.

Roll back to the previous binary:

    ssh pi 'cd /opt/mmm-native && ln -f mmm-native.prev mmm-native.rollback && mv -f mmm-native.rollback mmm-native'
    ssh -t pi 'sudo systemctl restart mmm-native'

## Roll back to the Chromium kiosk

    ssh -t pi 'sudo systemctl disable --now mmm-native'
    ssh pi 'cd /opt/stacks/mirror && docker compose up -d'
    ssh -t pi 'sudo systemctl enable --now dashboard-kiosk.service dashboard-monitor.service kiosk-page-watchdog.service dashboard-kiosk-nightly.timer'

Since 2026-11-11 those units and their scripts are no longer installed, and the desktop
autostart is reduced to `kanshi`. Before the last command, restore them from
`/home/max/backups/cleanup-2026-11-11/` on the Pi: the `.service`/`.timer` files to
`/etc/systemd/system/`, the two `.sh` files to `/usr/local/bin/`, `autostart` to
`/etc/xdg/labwc/autostart`, then `sudo systemctl daemon-reload`.

The web app ignores `spotify-tokens.json`, so re-connect Spotify in its settings page
if needed.

## Logs and status

    ssh pi 'journalctl -u mmm-native -f'      # app stderr goes to the journal
    ssh pi 'systemctl status mmm-native'

Behaviour of the unit:
- The app sends `READY=1` once its window is up, then `WATCHDOG=1` every 10 s from the
  UI thread.
- If the UI freezes for 90 s, systemd aborts the app and restarts it.
- Any exit is restarted after 60 s. This limits Polestar logins to one a minute in a
  crash loop, and the unit never gives up (`StartLimitIntervalSec=0`).
- A config change makes the app re-exec itself in place (same PID). That is not a
  restart.
