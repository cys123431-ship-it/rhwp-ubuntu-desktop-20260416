Phase-1 Ubuntu packaging assets for `rhwp`.

Included files:

- `rhwp.desktop`: desktop launcher with `%F` file association entry.
- `rhwp-mime.xml`: MIME registration for `.hwp` and `.hwpx`.

These assets are intended to be installed into:

- `/usr/share/applications/rhwp.desktop`
- `/usr/share/mime/packages/rhwp-mime.xml`

and then refreshed with:

```bash
update-desktop-database /usr/share/applications
update-mime-database /usr/share/mime
```
