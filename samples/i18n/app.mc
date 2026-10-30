import Toybox.Application;
import Toybox.Graphics;
import Toybox.WatchUi;

class I18nApp extends Application.AppBase {
  function initialize() {
    AppBase.initialize();
  }

  function getInitialView() {
    return [ new I18nView() ];
  }
}

class I18nView extends WatchUi.View {
  function initialize() {
    View.initialize();
  }

  function onUpdate(dc) {
    dc.setColor(Graphics.COLOR_WHITE, Graphics.COLOR_BLACK);
    dc.clear();

    var greeting = WatchUi.loadResource(Rez.Strings.Greeting);
    var cx = dc.getWidth() / 2;
    var cy = dc.getHeight() / 2;

    dc.setColor(Graphics.COLOR_WHITE, Graphics.COLOR_TRANSPARENT);
    dc.drawText(
      cx,
      cy,
      Graphics.FONT_LARGE,
      greeting,
      Graphics.TEXT_JUSTIFY_CENTER | Graphics.TEXT_JUSTIFY_VCENTER
    );
  }
}
