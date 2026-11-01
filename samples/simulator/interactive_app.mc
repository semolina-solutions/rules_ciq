import Toybox.Application;
import Toybox.Graphics;
import Toybox.System;
import Toybox.WatchUi;

class SimulatorInteractiveApp extends Application.AppBase {
  function initialize() {
    AppBase.initialize();
  }

  function getInitialView() {
    var view = new SimulatorInteractiveView();
    return [ view, new SimulatorInteractiveDelegate(view) ];
  }
}

class SimulatorInteractiveView extends WatchUi.View {
  var mPage = 1;
  var mSelectCount = 0;
  var mLastAction = "NONE";

  function initialize() {
    View.initialize();
  }

  function recordAction(act) {
    if (mLastAction.equals("NONE")) {
      mLastAction = act;
    } else {
      mLastAction = mLastAction + ">" + act;
    }
    System.println("interactive_app action=" + act + " page=" + mPage + " select=" + mSelectCount + " history=" + mLastAction);
  }

  function onShow() {
    var statusTitle = WatchUi.loadResource(Rez.Strings.StatusTitle);
    var devSettings = System.getDeviceSettings();
    var timeMode = devSettings.is24Hour ? "24H" : "12H";
    var unitMode = (devSettings.distanceUnits == System.UNIT_STATUTE) ? "STAT" : "METR";
    System.println("interactive_app ready: " + statusTitle + " [" + timeMode + "/" + unitMode + "]");
  }

  function onUpdate(dc) {
    dc.setColor(Graphics.COLOR_WHITE, Graphics.COLOR_BLACK);
    dc.clear();

    var statusTitle = WatchUi.loadResource(Rez.Strings.StatusTitle);
    var devSettings = System.getDeviceSettings();
    var timeMode = devSettings.is24Hour ? "24H" : "12H";
    var unitMode = (devSettings.distanceUnits == System.UNIT_STATUTE) ? "STAT" : "METR";

    var cx = dc.getWidth() / 2;
    var h = dc.getHeight();

    dc.setColor(Graphics.COLOR_BLUE, Graphics.COLOR_TRANSPARENT);
    dc.drawText(
      cx,
      h * 25 / 100,
      Graphics.FONT_XTINY,
      statusTitle + " [" + timeMode + "/" + unitMode + "]",
      Graphics.TEXT_JUSTIFY_CENTER | Graphics.TEXT_JUSTIFY_VCENTER
    );

    dc.setColor(Graphics.COLOR_WHITE, Graphics.COLOR_TRANSPARENT);
    dc.drawText(
      cx,
      h * 50 / 100,
      Graphics.FONT_SMALL,
      "PG " + mPage + " SEL " + mSelectCount,
      Graphics.TEXT_JUSTIFY_CENTER | Graphics.TEXT_JUSTIFY_VCENTER
    );

    dc.setColor(Graphics.COLOR_GREEN, Graphics.COLOR_TRANSPARENT);
    dc.drawText(
      cx,
      h * 75 / 100,
      Graphics.FONT_XTINY,
      mLastAction,
      Graphics.TEXT_JUSTIFY_CENTER | Graphics.TEXT_JUSTIFY_VCENTER
    );
  }
}

class SimulatorInteractiveDelegate extends WatchUi.BehaviorDelegate {
  var mView;

  function initialize(view) {
    BehaviorDelegate.initialize();
    mView = view;
  }

  function onNextPage() {
    mView.mPage += 1;
    mView.recordAction("NEXT");
    WatchUi.requestUpdate();
    return true;
  }

  function onPreviousPage() {
    mView.mPage -= 1;
    mView.recordAction("PREV");
    WatchUi.requestUpdate();
    return true;
  }

  function onSelect() {
    mView.mSelectCount += 1;
    var devSettings = System.getDeviceSettings();
    if (devSettings.isTouchScreen) {
      mView.recordAction("TAP");
    } else {
      mView.recordAction("SEL");
    }
    WatchUi.requestUpdate();
    return true;
  }

  function onMenu() {
    mView.recordAction("MENU");
    WatchUi.requestUpdate();
    return true;
  }

  function onBack() {
    mView.recordAction("BACK");
    WatchUi.requestUpdate();
    return true;
  }

  function onSwipe(evt) {
    var dir = evt.getDirection();
    if (dir == WatchUi.SWIPE_LEFT) {
      mView.recordAction("SWIPE_L");
    } else if (dir == WatchUi.SWIPE_RIGHT) {
      mView.recordAction("SWIPE_R");
    } else if (dir == WatchUi.SWIPE_UP) {
      mView.recordAction("SWIPE_U");
    } else if (dir == WatchUi.SWIPE_DOWN) {
      mView.recordAction("SWIPE_D");
    }
    WatchUi.requestUpdate();
    return true;
  }

  function onTap(evt) {
    var coords = evt.getCoordinates();
    mView.recordAction("TAP:" + coords[0] + "," + coords[1]);
    WatchUi.requestUpdate();
    return true;
  }
}
