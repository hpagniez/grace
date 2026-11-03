package orders;

import com.intuit.karate.junit5.Karate;

/** The JUnit runner `mvn test` uses; Grace runs the features itself and never loads it. */
class OrdersTest {
    @Karate.Test
    Karate all() {
        return Karate.run().relativeTo(getClass());
    }
}
