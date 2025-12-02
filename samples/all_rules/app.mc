import Toybox.Application;
import Toybox.WatchUi;

class SampleApp extends AppBase {
  function getInitialView() {
    return [ new SampleDataField() ];
  }
}

class SampleDataField extends SimpleDataField {
  function initialize() {
    SimpleDataField.initialize();
    label = "sample";
  }

  function compute(info) {
    System.println(Time.now().value() + ": computing...");
    return 42;
  }
}