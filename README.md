# 3bd
This is a project of making a user friendly debugger from scratch.
The name 3bd is an acronym for Three Body Debugger. I had an inspiration from the three body problem in mathematics/physics. However instead of using 3bd every time you would want to run it, it uses the acronym "tbd" without the number.
My main inspiration was that I have seen very little of user friendly debuggers or commercial software in general. I understand that most of these tools have a simple UI to make them version compatible, however I have been very displeased with the looks of most of these apps.

For example, I tried making the UI as customizable as possible, mainly by using color themes and movable widgets.

## Code
This project is written in the Rust programming language.
All of the source code is in the `code` directory, along with the assets (icons...).
There is a makefile in `build`, that produces the binary (runs `cargo build --release` and copies the binary to the dir). To install it, either move the executable into the `/bin` directory or add the build folder to your path.
In `docs` you will find the PDF documents that were used to create this project.
If you want to try out some of the features, you can use the examples in the `test` folders.

## Arch
This project is only for Linux, and limited to the AMD64 (x86_64) architecture (ARM or x86 might be possible later).
Works on both X11 and Wayland, was built on the Debian distro, (others are not tested).
It supports conventionally any compiled programming language that produces the DWARF data (compiling with the -g flag). I cannot guarantee every language will work, as some still dont have complete support of the DWARF standard (like Rust). So be prepared the program might not display all of the data correctly or might crash during loading of the data.
However, I tested thoroughly on languages C, C++ and Rust. You may also compile your code with the additional optimization flags, like `-Os` and more, and then debug the code. But keep in mind that the debugging data produced will be more limited due to these optimizations and therefore the tracing experience might seem illogical or strange.
I can process and display only what the debug information tells me.

Interpreted languages are unsupported, unless you want to debug the interpreter. Tracing child programs and threads also isn't supported. And debbuging TUI (Terminal User Interface) apps would display weird outputs in the terminal.

## Features
Basic and complete control over the execution of the tracee.
Breakpoints, tracking source and assembly code.
Reading memory, with multiple formats of displaying the data.
Displaying the registers.
Debugging of C, C++ and Rust files is tested and working.
Customizable UI (movable and resizable widgets), Sidebars and a Panel (also support for multiple widgets of the same type).