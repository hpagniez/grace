Feature: fixable and outline scenarios

Scenario: fixable
  * def flag = read('classpath:flag.json')
  * match flag.fixed == true

@smoke
Scenario: stays green
  * match 1 == 1

Scenario Outline: row <n>
  * match <n> == 1

  Examples:
    | n |
    | 1 |
    | 2 |

Scenario Outline: dynamic <n>
  * match n == 1

  Examples:
    | read('rows.json') |
