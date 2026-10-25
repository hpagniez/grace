@feat
Feature: tagged outline

@other
Scenario: other
  * match 1 == 1

@ol
Scenario Outline: speed <n>
  * match <n> == <n>

  @fast
  Examples:
    | n |
    | 1 |
    | 2 |

  @slow
  Examples:
    | n |
    | 3 |

  # not listed by a dry run, so it must never run
  @ignore
  Examples:
    | n |
    | 4 |
