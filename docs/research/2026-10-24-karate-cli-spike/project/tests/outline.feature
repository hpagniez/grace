Feature: outline with examples

Scenario Outline: add <a> and <b>
  * def sum = <a> + <b>
  * match sum == <expected>

  Examples:
    | a | b | expected |
    | 1 | 2 | 3        |
    | 2 | 3 | 5        |
    | 5 | 5 | 10       |
