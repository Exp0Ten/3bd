Dependencies and their respective installations :

rustup - for compiling the rust code
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```


zenity - for message dialogs (optional)

For Debian and Ubuntu
```sh
sudo apt install zenity
```

make - for easier building process
```sh
sudo apt install make
```
Otherwise compile using `cargo build --release` and run using `cargo run --release` inside the `code` dir.

gcc & g++ - for compiling the example codes (c and c++ respectively)
```sh
sudo apt install gcc
sudo apt install g++
```

Lastly, this project uses the glibc:
```sh
sudo apt install libc6
```
(libc6 is the current version of the glibc, and should also be the dependency of mentioned compilers)
