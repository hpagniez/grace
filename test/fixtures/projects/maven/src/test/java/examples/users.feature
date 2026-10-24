Feature: maven layout

Scenario: config, classpath data and classpath call
  * match fromConfig == 'cfg'
  * match read('classpath:data/user.json').name == 'kid'
  * def r = call read('classpath:common/helper.feature') { who: 'x' }
  * match r.greeting == 'hello x'
