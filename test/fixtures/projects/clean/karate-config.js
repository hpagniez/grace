function fn() {
  var env = karate.env;
  karate.log('karate-config.js running, env =', env);
  var mock = karate.callSingle('classpath:start-mock.feature');
  return { env: env, baseUrl: 'http://localhost:' + mock.port, devFlag: false };
}
