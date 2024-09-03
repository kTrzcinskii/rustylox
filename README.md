# RustyLox

## Description
RustyLox is a bytecode lox interpreter written entirely in Rust. It was created following guidance from second part of the marvelous book ["Crafting Interpreters" by Robert Nystrom](https://craftinginterpreters.com/). (I also created simpler interpreter in C#, which goal was to make me familiar with basic interpreters world - you can check [my SharpLox project here](https://github.com/kTrzcinskii/sharplox).) The language that author used for his bytecode interpreter was C, so naturally my approach was quite different - I was trying to follow all rust's best practices (which means we don't like global variables, bare pointers and tons of unsafe behaviour). I also added some more feauters and changed existing ones, so that project was actually mine, not just reimplementation of something already existing.

## Motivation
As I already mentioned, I've already created lox interpreter in C#. It taught me a lot of valuable skills and gave me a broad knowledge, but when it comes to performance, let's just say it wasn't the fastest piece of software I've seen. In this project, I wanted not only to create proper Lox interpreter, but also make it much faster. My second motivation was desire to level up my rust skills - after 5k lines of rust code in this project I can certainly say that indeed my skills in this language are much better (but there is still a lot of room for improvement). Last but not least, it was just a lot of fun to implement such low level features as my own virtual machine, bytecode instructions and even some simple hash table. 

## Implementation
Project is split into modules, which are (in alphabetical order):
 - chunk - there lives logic behind virtual machine bytecode instructions. I implemented all the parsing/serializing by myself just for practice. There is also `chunk` struct, which holds instructions and constants and gives an API for managing them inside the chunk.
 - compiler - the biggest module in the project, this is where stream of tokens is transformed into stream of bytecode instructions. It uses Vaughan Pratt's "top-down operator precedence parsing".
 - error - small file with proper error codes in case of invalid program / runtime error
 - lexer - the first element of our pipeline, this is where the source code is turned into stream of tokens
 - logger - utility module for logging debug information
 - native_functions - module with implementation of native lox function, I only created (just for example purposes) - `clock`
 - table - my own simple hash table representation. I know there is already existing, ready to use rust's HashMap, but I thought it was a good learning experience to implement one by hand
 - value - representation of all different Lox values (numbers, booleans, nil, strings, etc.). This is the only place in the code where I had to use `unsafe` keyword, as I was playing with `unions` there.
 - vm - second biggest module in the project, probably the most important one - the heart of this interpreter, virtual machine. It reads bytecode instructions and properly executes them.

## Examples
There are some examples in the `examples` directory. Some of them are created entirely by me, some are from the book and test some weird edge cases. You can run them by hand, or if you want there is script `run_examples.sh` that will run all of them.

## Usage
RustyLox can run in two modes:

#### Interpreting a file
 - usage: `./rustylox [filename]`
 It interprets file with lox code. 

#### Interpreting prompt
 - usage: `./rustylox`
 It waits for user input and interprets it. Globals are shared between prompts, so when you run:
 `var a = 5;`
 and the next line you type:
 `print a;`
 RustyLox will properly get the value of global variable a.

(I explicitly show this feature, as I had some refactoring to do in my code to make it work due to the rust's strict ownership rules)