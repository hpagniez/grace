function fn() {
  var File = Java.type('java.io.File');
  return {
    dbUrl: 'jdbc:sqlite:' + new File('target/orders.db').getAbsolutePath(),
    users: Java.type('support.Users')
  };
}
