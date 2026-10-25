Feature: outline rows in unusual tables

Scenario Outline: add <a>
  * def sum = <a> + 1
  * match sum == <a> + 1

  Examples:
    | a |
    # | 0 |
    | 1 |

    | 2 |
    | 3 |

Scenario Outline: dynamic <a>
  * match a == '#number'

  Examples:
    | read('rows.json') |
