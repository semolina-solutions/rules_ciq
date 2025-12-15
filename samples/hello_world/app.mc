import Toybox.Application;
import Toybox.WatchUi;

class HelloWorldApp extends AppBase {
  function getInitialView() {
    return [ new HelloWorldDataField() ];
  }
}

class HelloWorldDataField extends SimpleDataField {
  function initialize() {
    SimpleDataField.initialize();
    label = "Hello World";
  }

  function compute(info) {
    System.println(Time.now().value() + ": computing...");
    return 42;
  }
}