I did not expect to have dependencies, but i found my first one and so this file was created. Later on I will fix it in a more readable state.

Dependencies and their respective installations :

rustup - for compiling the rust code
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

zenity - for message dialogs (technically not necessary)

For Debian an Ubuntu (more on their git repo)
```sh
sudo apt install zenity
```