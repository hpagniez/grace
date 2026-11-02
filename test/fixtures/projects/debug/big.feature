Feature: big

  Scenario: big value
    * def big = karate.repeat(102400, function(){ return 'x' }).join('')
    * def token = karate.properties['token']
    * print 'after'
