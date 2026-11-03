Feature: a SQLite driver from the build, and HTTP through Karate's own client and server

  Scenario: insert and read a user
    * def DriverManager = Java.type('java.sql.DriverManager')
    * def conn = DriverManager.getConnection(dbUrl)
    * def st = conn.createStatement()
    * st.executeUpdate('create table if not exists users (name text)')
    * st.executeUpdate('delete from users')
    * st.executeUpdate("insert into users values ('ada')")
    * def rs = st.executeQuery('select name from users')
    * rs.next()
    * def name = rs.getString('name')
    * conn.close()
    * match name == 'ada'

  Scenario: call a local mock
    * def server = karate.start({ mock: 'mock.feature' })
    * url 'http://localhost:' + server.port
    * path 'hello'
    * method get
    * status 200
    * match response == { hello: 'world' }
