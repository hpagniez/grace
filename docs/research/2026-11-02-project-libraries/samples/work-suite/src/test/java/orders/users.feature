Feature: the company library and the driver, through a compiled helper

  Scenario: add a user through the company library
    * match users.addAndRead(dbUrl, 'ada') == 'ada'
