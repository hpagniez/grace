Feature: slow scenarios

Background:
  * def Thread = Java.type('java.lang.Thread')

@slow
Scenario: slow loop
  * Thread.sleep(1000)
  * print 'tick 1'
  * Thread.sleep(1000)
  * print 'tick 2'
  * Thread.sleep(1000)
  * print 'tick 3'
  * Thread.sleep(1000)
  * print 'tick 4'
  * Thread.sleep(1000)
  * print 'tick 5'
  * Thread.sleep(1000)
  * print 'tick 6'
  * Thread.sleep(1000)
  * print 'tick 7'
  * Thread.sleep(1000)
  * print 'tick 8'

@smoke @slow
Scenario: smoke and slow
  * Thread.sleep(500)
  * match 1 == 1
