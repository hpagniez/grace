function fn() {
  // Java interop in the config: Karate 1 projects often build values this way
  var File = Java.type('java.io.File');
  return { dbUrl: 'jdbc:sqlite:' + new File('test.db').getAbsolutePath() };
}
