# Reed Pal
[中文](README.zh-cn.md)

Reed Pal is a program testing tool with concise config syntax. It can find bugs in your program("user program") by checking its output with expected output(which can be written to test config file or get from "standard program") in given input(with glob expansion and random generation supported). Reed Pal runs test in parallel to greatly reduce time cost. Reed Pal also supports "session", so you can get detailed info of every failed tests after test finished, and retest after fixing bugs.

# Usage
## Run test

For how to specify compiler, compiler arguments, timeout, see `rpal --help`

For details of test config syntax, see [Test config](#test-config)

### Check (comparing with expected output defined in test config)

1. prepare source file: `wa.c`(we expected to print `x` instead of `x + 1`)
```C
#include <stdio.h>

int main(void) {
  int x = 0;
  scanf("%d", &x);
  printf("%d", x + 1);
}
```
2. prepare test config: `wa.test`
The following config describes:

|input|expected output|
|----|----|
|0|0|
|1|1|
|2|2|

```
----
0
----
0
----
1
----
1
----
2
----
2
----
```
3. run test(compile using `gcc -Wall -Wextra -lm` by default)
```
$ rpal check wa.c
Running on: linux, CPU cores: 16
Data directory: ~/.local/share/reed_pal
Session id: ebea342f-2f24-4441-93f4-ccba748063b9
Current working directory: /tmp/tests/pal/check
Running for type: Check
Parsing config...
Job count: 3
Compiling using: gcc -Wall -Wextra -lm
Running jobs using 3 threads...
Test info directory: /tmp/tests/pal/check/tests_info/wa
A "." indicates a passed test. A "X" indicates a failed test: 
XXX
Saving test result to ~/.local/share/reed_pal/wa_store.json...
FAILED: pass = 0, fail = 3
time: 70ms(total) = 0ms(parse) + 69ms(compile) + 1ms(run)
```

For how to specify test config, see `rpal check --help`

### Pal (comparing with output from "standard program")

1. prepare source file: `success.c`(which would pass the test)
```C
#include <stdio.h>

int main(void) {
  int x = 0;
  scanf("%d", &x);
  printf("%d", x + 1);
}
```
2. prepare standard program file: `success_std.c`
```C
#include <stdio.h>

int main(void) {
  int x = 0;
  scanf("%d", &x);
  printf("%d", x + 1);
}
```
3. prepare test config: `success.test`
The following config describes("expand" `[1-10000]`, `[2100000000-2100001000]` to multiple inputs):

|input|
|----|
|1|
|2|
|3|
|4|
|5|
|...|
|9998|
|9999|
|10000|
|2100000000|
|2100000001|
|2100000002|
|...|
|2100000998|
|2100000999|
|2100001000|

```
glob
----
[1-10000]
----
[2100000000-2100001000]
----
```
4. run test(compile using `gcc -Wall -Wextra -lm` by default)
```
$ rpal pal success.c
Running on: linux, CPU cores: 16
Data directory: ~/.local/share/reed_pal
Session id: 0bf4b8b3-cef1-40b8-a69b-cf4c4fe7b9ed
Current working directory: /tmp/tests/pal/pal
Running for type: Pal
Parsing config...
Job count: 11001
Compiling using: gcc -Wall -Wextra -lm
Running jobs using 16 threads...
Test info directory: /tmp/tests/pal/pal/tests_info/success
A "." indicates a passed test. A "X" indicates a failed test: 
................................................................................................
// snip
.........................................................
Saving test result to ~/reed_pal/success_store.json...
PASSED: pass = 11001, fail = 0
time: 2560ms(total) = 1419ms(parse) + 64ms(compile) + 1077ms(run)
```
For how to specify test config, standard program source, see `rpal pal --help`

### RandomPal (comparing with output from "standard program", with input generated from random value with in range)

Sometimes, the possible range of input is very wide, making it impossible to enumerate each value. To make tests possible, RandomPal chooses random value from the possible range and limits total number of tests.

1. prepare source file: `wa.c`(which would pass the test only when a < 10000)
```C
#include <stdio.h>

int main(void) {
  int a = 0;
  scanf("%d", &a);
  if (a < 10000) {
    printf("%d", a + 1);
  } else {
    printf("%d", a);
  }
}
```
2. prepare standard program file: `wa_std.c`
```C
#include <stdio.h>

int main(void) {
  int x = 0;
  scanf("%d", &x);
  printf("%d", x + 1);
}
```
3. prepare test config: `wa.test`
The following config describes: random sample 5000 times from `[1-10000]` and 5000 times from `[2100000000-2100001000]`, 10000 times in total.
```
10000
----
[1-10000]
----
[2100000000-2100001000]
----
```
4. run test(compile using `gcc -Wall -Wextra -lm` by default)
```
$ rpal random-pal wa.c
Running on: linux, CPU cores: 16
Data directory: ~/.local/share/reed_pal
Session id: 2250f3aa-a52e-4397-a9f2-e2c79c9d7de5
Current working directory: /tmp/tests/pal/random_pal
Running for type: RandomPal
Parsing config...
Job count: 10000
Compiling using: gcc -Wall -Wextra -lm
Running jobs using 16 threads...
Test info directory: /tmp/tests/pal/random_pal/tests_info/wa
A "." indicates a passed test. A "X" indicates a failed test: 
......................................................................................
// snip
XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
Saving test result to ~/.local/share/reed_pal/wa_store.json...
FAILED: pass = 4999, fail = 5001
time: 2382ms(total) = 1338ms(parse) + 74ms(compile) + 970ms(run)
```

For how to specify test config, standard program source, see `rpal random-pal --help`

## Using session to get info of failed tests
```
$ rpal session
Running on: linux, CPU cores: 16
Data directory: ~/.local/share/reed_pal
Session id: 2250f3aa-a52e-4397-a9f2-e2c79c9d7de5
Reading results from: ~/.local/share/reed_pal/wa_store.json...
PASSED: 4999, FAILED: 5001
Of failed tests: 
WA: 5001
```

### using `session load` to load input/output
```
$ rpal session load
Running on: linux, CPU cores: 16
Data directory: ~/.local/share/reed_pal
Session id: 2250f3aa-a52e-4397-a9f2-e2c79c9d7de5
Reading results from: ~/.local/share/reed_pal/wa_store.json...
job_type: WA
WA(Job id = 344)
Input file: /tmp/pal/random_pal/tests_info/wa/344/in.txt
Actual output file: /tmp/tests/pal/random_pal/tests_info/wa/344/actual_out.txt
Expected output file: /tmp/tests/pal/random_pal/tests_info/wa/344/expected_out.txt
```
For how to specify number of tests to load or type of failure, see `rpal session load --help`

### using `session continue` to retest failed tests after fixing bugs

Note that a new session would be created, and info of previous session WOULD BE LOST.

```
$ rpal session continue
Running on: linux, CPU cores: 16
Data directory: ~/.local/share/reed_pal
Session id: 2250f3aa-a52e-4397-a9f2-e2c79c9d7de5
Reading results from: ~/.local/share/reed_pal/wa_store.json...
Retesting...
Job count: 5000
Compiling using: gcc -Wall -Wextra -lm
Running jobs using 16 threads...
Test info directory: /tmp/tests/pal/random_pal/tests_info/wa
A "." indicates a passed test. A "X" indicates a failed test: 
..........................................................................................................
// snip
.................................................................................
Saving test result to ~/.local/share/reed_pal/wa_store.json...
FAILED: pass = 4991, fail = 9
time: 303ms(total) = 0ms(parse) + 39ms(compile) + 264ms(run)
```

### using `session retest` to retest passed and failed tests after fixing bugs

Note that a new session would be created, and info of previous session WOULD BE LOST.


```
$ rpal session retest
Running on: linux, CPU cores: 16
Data directory: ~/.local/share/reed_pal
Session id: 2250f3aa-a52e-4397-a9f2-e2c79c9d7de5
Reading results from: ~/.local/share/reed_pal/wa_store.json...
Retesting...
Job count: 5000
Compiling using: gcc -Wall -Wextra -lm
Running jobs using 16 threads...
Test info directory: /tmp/tests/pal/random_pal/tests_info/wa
A "." indicates a passed test. A "X" indicates a failed test: 
.....................................................
// snip
............................................................X.XX.XXXXXX
Saving test result to ~/.local/share/reed_pal/wa_store.json...
FAILED: pass = 4991, fail = 9
time: 282ms(total) = 0ms(parse) + 33ms(compile) + 249ms(run)
```

# Test config
## Check
File structure:
- separator used to separate input and expected output. You can set your own separator to use in the first line. 
- input(can contain multiple lines)
- separator
- expected output(can contain multiple lines)
- separator
- ...
- separator
- input
- separator
- expected output
- separator
...

Example: 
```
----
0
----
1
----
1
----
2
----
2
----
3
----
```
would be parsed to:

|input|expected output|
|----|----|
|0|1|
|1|2|
|2|3|

## Pal
File structure:
- type ("simple" or "glob", see below)
- separator
- input
- separator
- input
- ...
- separator
- input
- separaotr

### Type: simple
inputs are passed to program "as is".

Example: 

```
simple
----
[1-10000]
----
[2100000000-2100001000]
----
```

would be parsed to:

- `[1-10000]`
- `[2100000000-2100001000]`

### Type: glob
inputs are "expanded" to all possible values before passed to program.

To pass `[`, `]`, `-` to program, use `\[`, `\]`, `\-`.

Two patterns are supported:
- `[xyz\-\[]` -> `x`, `y`, `z`, `-`, `[`(can contain any character)
- `[1-5]` -> `1`, `2`, `3`, `4`, `5`(must be `[number-number]`)

Example: 

```
glob
---
abc[xyz]k
---
a[1-3]b[2-5]z
---
```
would be parsed to:

- `abcxk`
- `abcyk`
- `abczk`
- `a1b2z`
- `a1b3z`
- `a1b4z`
- `a1b5z`
- `a2b2z`
- `a2b3z`
- `a2b4z`
- `a2b5z`
- `a3b2z`
- `a3b3z`
- `a3b4z`
- `a3b5z`

## RandomPal
File structure:
- number of inputs to generate from glob
- separator
- input
- separator
- input
- ...
- separator
- input
- separaotr

Example:
```
6
---
abc[xyz]k
---
a[1-3]b[2-5]z
---
```

would be (possibly) parsed to:
- `abcxk`
- `abcxk`
- `abczk`
- `a1b3z`
- `a1b5z`
- `a2b2z`

# Build
Reed Pal is written in Rust, so a Rust installation is needed.

To build Reed Pal:
```
$ git clone https://github.com/ReedThree/rpal
$ cd rpal
$ cargo build --release
$ ./target/release/rpal --version
```