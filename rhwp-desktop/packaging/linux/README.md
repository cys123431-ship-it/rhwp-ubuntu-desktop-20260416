Phase-1 Ubuntu packaging assets for Geulbit X.

Included files:

- `rhwp.desktop`: desktop launcher with `%F` file association entry.
- `rhwp-mime.xml`: MIME registration for `.hwp` and `.hwpx`.
- `postinst`: refreshes desktop, MIME, and icon caches after install.
- `postrm`: refreshes desktop, MIME, and icon caches after removal.

These assets are intended to be installed into:

- `/usr/share/applications/rhwp.desktop`
- `/usr/share/mime/packages/rhwp-mime.xml`

and then refreshed with:

```bash
update-desktop-database /usr/share/applications
update-mime-database /usr/share/mime
```

The Debian bundle wires these commands through maintainer scripts so the caches are refreshed on install and removal when the helper tools are present.
