This folder is named like the `@fast` tag on purpose. Karate parses its command line with picocli,
which reads an argument starting with `@` as an argument file: `-t @fast` run from the project root
would try to read this folder and fail. The app passes `-t=@fast`, which picocli does not expand.
