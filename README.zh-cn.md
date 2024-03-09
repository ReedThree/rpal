# Reed Pal

Reed Pal可用于测试编写的程序，寻找其中的bug。除了手动编写输入/预期输出外，还可与标准程序的输出进行比较（“对拍”），且可通过简洁的语法批量生成输入数据。Reed Pal通过多线程运行测试，极大减少了测试所用的时间。此外，通过“会话”(Session)保存测试结果，可在测试结束后获取输入、预期输出、实际输出等信息，并在修复bug后重新运行之前的测试。

# 使用
## 进行测试

若要修改编译器、编译器参数、单个测试超时时间，参见`rpal --help`。

有关测试配置文件的语法，参见[测试配置文件](#测试配置文件)

### Check (预先在测试配置文件中指定期望输出)

1. 准备欲测试程序的源文件：`wa.c`(假定我们期望程序打印`x`而非`x + 1`， 故该程序不能通过测试)
```C
#include <stdio.h>

int main(void) {
  int x = 0;
  scanf("%d", &x);
  printf("%d", x + 1);
}
```
2. 准备测试配置文件：`wa.test`
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
以上测试配置文件描述了如下输入、期望输出：

|input|expected output|
|----|----|
|0|0|
|1|1|
|2|2|

3. 进行测试(默认将使用`gcc -Wall -Wextra -lm`编译)
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

要单独指定测试配置文件名，参见：`rpal check --help`。

### Pal (将输出与标准程序的输出对比)

1. 准备欲测试程序源文件：`success.c`(本例中该程序将通过测试)
```C
#include <stdio.h>

int main(void) {
  int x = 0;
  scanf("%d", &x);
  printf("%d", x + 1);
}
```
2. 准备标准程序的源文件：`success_std.c`
```C
#include <stdio.h>

int main(void) {
  int x = 0;
  scanf("%d", &x);
  printf("%d", x + 1);
}
```
3. 准备测试配置文件：`success.test`
```
glob
----
[1-10000]
----
[2100000000-2100001000]
----
```
以上测试配置文件将生成如下输出（将“展开”`[1-10000]`、`[2100000000-2100001000]`为多个输入数据）：
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

4. 进行测试(默认将使用`gcc -Wall -Wextra -lm`编译)
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
// (省略部分输出)
.........................................................
Saving test result to ~/reed_pal/success_store.json...
PASSED: pass = 11001, fail = 0
time: 2560ms(total) = 1419ms(parse) + 64ms(compile) + 1077ms(run)
```
要指定测试配置文件名、标准程序源文件，参见`rpal pal --help`。

### RandomPal (将输出与标准程序的输出对比，但输入在指定范围内随机生成，而非遍历)

有时，可能的输入范围非常大，故不可能遍历每一个输入。通过随机在输入范围内选取数据作为输入，可减小所需的测试数量，同时仍能在大概率下找到bug。

1. 准备源文件：`wa.c`(本例中，只有在a < 10000时才能通过测试)
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
2. 准备标准程序的源文件：`wa_std.c`
```C
#include <stdio.h>

int main(void) {
  int x = 0;
  scanf("%d", &x);
  printf("%d", x + 1);
}
```
3. 准备测试配置文件：`wa.test`
```
10000
----
[1-10000]
----
[2100000000-2100001000]
----

这将随机生成10000个输入数据，其中5000个在`[1-10000]`中1采样，5000个在`[2100000000-2100001000]`中采样。

```
4. 进行测试(默认将使用`gcc -Wall -Wextra -lm`编译)
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
......................................................................................................................................................................................................................................................................................................................................................X..............................................................................................................................................................
// (省略部分输出)
XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
Saving test result to ~/.local/share/reed_pal/wa_store.json...
FAILED: pass = 4999, fail = 5001
time: 2382ms(total) = 1338ms(parse) + 74ms(compile) + 970ms(run)
```

要指定测试配置文件名、标准程序源文件，参见`rpal random-pal --help`。

## 通过会话获取未通过测试的信息
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

### 使用`session load`可获取对应的输入/输出
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

要指定加载的测试结果数量、未通过原因，参见`rpal session load --help`。

### 使用`session continue`在修复bug后重新进行之前未通过的测试

注意：这将创建一个新的会话，之前的会话将会**丢失**。

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
// (省略部分输出)
.................................................................................
Saving test result to ~/.local/share/reed_pal/wa_store.json...
FAILED: pass = 4991, fail = 9
time: 303ms(total) = 0ms(parse) + 39ms(compile) + 264ms(run)
```

### 使用`session retest`在修复bug后重新进行之前的所有测试

注意：这将创建一个新的会话，之前的会话将会**丢失**。

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
// (省略部分输出)
............................................................X.XX.XXXXXX
Saving test result to ~/.local/share/reed_pal/wa_store.json...
FAILED: pass = 4991, fail = 9
time: 282ms(total) = 0ms(parse) + 33ms(compile) + 249ms(run)
```

# 测试配置文件
## Check
文件结构：
- 分隔符：用于分隔输入和期望输出。可在第一行自定义分隔符，只要整个文件使用相同的分隔符即可。
- 输入(可包含多行)
- 分隔符
- 期望输出(可包含多行)
- 分隔符
- ...
- 分隔符
- 输入
- 分隔符
- 期望输出
- 分隔符
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
将被解析为：

|输入|期望输出|
|----|----|
|0|1|
|1|2|
|2|3|

## Pal
文件结构：
- 类型 ("simple"或"glob"，详见下)
- 分隔符
- 输入(可包含多行)
- 分隔符
- 输入
- ...
- 分隔符
- 输入
- 分隔符

### 类型: simple

输入将原样传递给被测试程序。

Example: 

```
simple
----
[1-10000]
----
[2100000000-2100001000]
----
```

将被解析为：

- `[1-10000]`
- `[2100000000-2100001000]`

### 类型: glob
配置文件中的单个输入将被“扩展”成多个输入，再传递给程序。

要传递`[`, `]`, `-`， 需转义：`\[`, `\]`, `\-`。

支持如下两种“扩展”模式：
- `[xyz\-\[]` -> `x`, `y`, `z`, `-`, `[`(`[]`中可包含任意字符)
- `[1-5]` -> `1`, `2`, `3`, `4`, `5`(必须为`[数字-数字]`)

Example: 

```
glob
---
abc[xyz]k
---
a[1-3]b[2-5]z
---
```

将被解析为：

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
文件结构：
- 根据指定的范围随机生成的输入数量
- 分隔符
- 输入(可包含多行)
- 分隔符
- 输入(可包含多行)
- ...
- 分隔符
- 输入(可包含多行)
- 分隔符

Example:
```
6
---
abc[xyz]k
---
a[1-3]b[2-5]z
---
```

可能会被解析为：
- `abcxk`
- `abcxk`
- `abczk`
- `a1b3z`
- `a1b5z`
- `a2b2z`

# 编译
Reed Pal由Rust编写，故编译需要安装Rust工具链。

要编译Reed Pal：
```
$ git clone https://github.com/ReedThree/rpal
$ cd rpal
$ cargo build --release
$ ./target/release/rpal --version
```