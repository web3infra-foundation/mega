# Troublesshooting
## How to deploy actions on ubuntu runners
We use `sudo` command to run `apt/apt-get` and install systemdependencies. If you are using a un-priviledged user account, the easist way is edit `sudoers`.
- You should switch to root and enter:
```bash
visudo
```
- Then add this line into it:
```txt
%your_user_name ALL=(ALL:ALL) NOPASSWD:/usr/bin/apt,/usr/bin/apt-get
```
