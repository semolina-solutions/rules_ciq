import Toybox.Application;
import Toybox.Application.Properties;
import Toybox.Graphics;
import Toybox.System;
import Toybox.WatchUi;

class SimulatorDataFieldApp extends Application.AppBase {
  function initialize() {
    AppBase.initialize();
  }

  function getInitialView() {
    return [ new SimulatorDataField() ];
  }

  function onSettingsChanged() {
    WatchUi.requestUpdate();
  }
}

class SimulatorDataField extends WatchUi.DataField {
  var mTimerState = "STOPPED";
  var mLaps = 0;
  var mHeartRate = 0;

  function initialize() {
    DataField.initialize();
  }

  function onTimerStart() {
    mTimerState = "RUNNING";
  }

  function onTimerPause() {
    mTimerState = "PAUSED";
  }

  function onTimerResume() {
    mTimerState = "RUNNING";
  }

  function onTimerStop() {
    mTimerState = "STOPPED";
  }

  function onTimerLap() {
    mLaps += 1;
  }

  function onWorkoutStepComplete() {
    mLaps += 10;
  }

  function compute(info) {
    if (info != null && info has :currentHeartRate && info.currentHeartRate != null) {
      mHeartRate = info.currentHeartRate;
    }
    var title = Properties.getValue("title");
    var targetHr = Properties.getValue("targetHr");
    System.println("data_field compute: title=" + title + " targetHr=" + targetHr + " state=" + mTimerState + " laps=" + mLaps + " hr=" + mHeartRate);
  }

  function onUpdate(dc) {
    var bgColor = getBackgroundColor();
    var fgColor = (bgColor == Graphics.COLOR_BLACK) ? Graphics.COLOR_WHITE : Graphics.COLOR_BLACK;
    var accentColor = (bgColor == Graphics.COLOR_BLACK) ? Graphics.COLOR_BLUE : Graphics.COLOR_DK_BLUE;

    dc.setColor(fgColor, bgColor);
    dc.clear();

    var title = Properties.getValue("title");
    var targetHr = Properties.getValue("targetHr");

    var cx = dc.getWidth() / 2;
    var h = dc.getHeight();

    dc.setColor(accentColor, Graphics.COLOR_TRANSPARENT);
    dc.drawText(
      cx,
      h * 22 / 100,
      Graphics.FONT_XTINY,
      title + " (" + targetHr + ")",
      Graphics.TEXT_JUSTIFY_CENTER | Graphics.TEXT_JUSTIFY_VCENTER
    );

    dc.setColor(fgColor, Graphics.COLOR_TRANSPARENT);
    dc.drawText(
      cx,
      h * 52 / 100,
      Graphics.FONT_MEDIUM,
      "HR " + mHeartRate,
      Graphics.TEXT_JUSTIFY_CENTER | Graphics.TEXT_JUSTIFY_VCENTER
    );

    dc.drawText(
      cx,
      h * 80 / 100,
      Graphics.FONT_XTINY,
      mTimerState + " L:" + mLaps,
      Graphics.TEXT_JUSTIFY_CENTER | Graphics.TEXT_JUSTIFY_VCENTER
    );
  }
}

