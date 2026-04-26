# systemd Packaging

Reference systemd units for production-style local deployment.

The service expects the runtime binary at:

```text
/opt/geniepod/bin/genie-home-runtime
```

It serves the local runtime socket at:

```text
/run/geniepod/home-runtime.sock
```

Persistent runtime files are stored under:

```text
/var/lib/geniepod/
```

The service writes separate durable files for entity state, actuation audit,
and runtime events.

Install sketch:

```bash
sudo groupadd --system geniepod || true
sudo useradd --system --gid geniepod --home-dir /var/lib/geniepod --shell /usr/sbin/nologin geniepod || true
sudo install -m 0755 target/release/genie-home-runtime /opt/geniepod/bin/genie-home-runtime
sudo install -m 0644 packaging/systemd/genie-home-runtime.service /etc/systemd/system/genie-home-runtime.service
sudo install -m 0644 packaging/systemd/genie-home-runtime.tmpfiles /etc/tmpfiles.d/genie-home-runtime.conf
sudo systemd-tmpfiles --create /etc/tmpfiles.d/genie-home-runtime.conf
sudo systemctl daemon-reload
sudo systemctl enable --now genie-home-runtime.service
```

`genie-claw` should connect through the Unix socket, not through a LAN-exposed
HTTP endpoint.
