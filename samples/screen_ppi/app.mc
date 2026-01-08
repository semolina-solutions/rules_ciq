import Toybox.Application;
import Toybox.WatchUi;

import GeneratedCode;

class SampleApp extends Application.AppBase {
  function getInitialView() {
    return [ new SampleView() ];
  }
}

class SampleView extends WatchUi.View {
  var inch;
  
  function initialize() {
    WatchUi.View.initialize();
  }

  function onLayout(dc) {
    inch = loadResource(Rez.Drawables.Inch);
  }

  function onUpdate(dc) {
    dc.setColor(Graphics.COLOR_WHITE, Graphics.COLOR_WHITE);
    dc.clear();

    var w = dc.getWidth();
    var h = dc.getHeight();

    dc.drawBitmap(
      (w - inch.getWidth()) / 2,
      (h - inch.getHeight()) / 2,
      inch
    );
  }
}