# 3bd
This is a project of making a user friendly debugger from scratch.
The name 3bd is an acronym for Three Body Debugger. I had an inspiration from the three body problem in mathematics/physics. However instead of using 3bd every time you would want to run it, instead it uses a acronym "tbd" without the number.
I also wish to somehow reflect that in the UI feel, however that is a secondary objective.
My main inspiration was that I have seen very little user friendly debuggers or commercial software in general. I understand that most of these tools have a simple UI to make them version compatible, however I have been very displeased with the looks of most of these apps.

I have played many puzzle games with programming aspects and loved the UI that was used for debugging or visualizing the software, and was very disappointed to find that commercial software lacks some basic theming options.
I wish to therefore make it customizable enough and give the user the control over how they use the software. In the end, its just a piece of code running complex algorithms and it'd be sad if someone chose to not use the tools just because of the outlook of the software.

## Code
Ideally, everything will be written in Rust, as I am a big fan of this language, mainly for its stability and speed.

## Arch
This project is only for Linux, and limited to the AMD64 (x86_64) architecture (maybe more arches will be possible). I have thought of making this program portable to different architectures using config files that specify the functionality/implementation for the processors, but decided that would be close to impossible.
The second important thing is supported languages. C and Assembly are a given, as they are essential for general debugging. However, I won't be making language specific profile, meaning you will not be able to debug the languages for mistakes within that language, but only on the general level of runtime. (I am yet to see what the debug symbols tell you, meaning language specific errors might be possible)
Source code tracking will be a feature though, and I plan on dynamically displaying the variables, maybe linking them as well to the higher level code (like enums). Interpreted languages are unsupported, unless you wanna debug the interpreter.

## Features
Basic and complete control over the child programs execution.
Breakpoints, tracking source and assembly code.
Dynamically interactive memory reading and label tracking and processing (maybe connecting to varibles and their types).
Colors and types to make notes or see changes.
Display of processor state (registers, pointers).
C and Assembly support (maybe some easy high level code profiling using config files).
Customizable UI (Displays, Terminal, Controls, Settings) + CLI version, all using config files.
Complete documentation WITH mentions to the source code.


## Disclaimer
This project is for my finals and therefore I will have to write a lot of the documentation in Czech. I am open to translations, but all of those will be in designated folders.
