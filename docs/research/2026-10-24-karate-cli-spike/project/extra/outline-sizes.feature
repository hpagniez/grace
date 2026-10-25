Feature: outline sizes

Scenario Outline: one <a>
  * def x = <a>

  Examples:
    | a |
    | 1 |

Scenario Outline: five <a>
  * def x = <a>

  Examples:
    | a |
    | 1 |
    | 2 |
    | 3 |
    | 4 |
    | 5 |

  @tagged
  Examples:
    | a |
    | 6 |
    | 7 |
