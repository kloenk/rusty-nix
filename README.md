# rusty-nix
Nix written in rust (this will take some time if it will ever finish)

# Hacking
Create private store which can substitute.
This needs to run the rusty daemon in the created shell, as it has a private mount space.
~~~sh
sudo unshare -m
mkdir -p /rusty-nix/{up,work}
mount -t overlay overlay -o lowerdir=/nix/,upperdir=/rusty-nix/up,workdir=/rusty-nix/work/ /nix/
~~~

Show socket trafic as hex
~~~sh
sudo socat -t100 -x -v UNIX-LISTEN:/tmp/nix-store,mode=777,reuseaddr,fork UNIX-CONNECT:/home/kloenk/daemon.socket
~~~
