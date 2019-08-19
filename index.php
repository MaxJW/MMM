<!DOCTYPE html>
<html>

<head>
  <title>MMM</title>
  <meta http-equiv="refresh" content="1800" />
  <meta http-equiv="content-type" content="text/html; charset=utf-8" />
  <script src="js/jquery-3.4.1.min.js"></script>
  <script src="js/moment.min.js"></script>
  <script src="js/simpleslider.min.js"></script>

  <script src="https://kit.fontawesome.com/d368698d9e.js"></script>

  <link rel="stylesheet" type="text/css" href="css/style.min.css" />
  <link rel="stylesheet" type="text/css" href="css/weather-icons.min.css" />

  <link href="https://fonts.googleapis.com/css?family=Roboto+Condensed:300,400&display=swap" rel="stylesheet" />

  <script src="config.js" type="text/javascript"></script>
  <script src="modules/weather/openWeather.min.js" type="text/javascript"></script>
  <script src="modules/rss/rss.min.js" type="text/javascript"></script>
  <script src="modules/calendar/jquery-google-calendar-events.min.js" type="text/javascript"></script>
  <script src="modules/spotify/nowplaying.js" type="text/javascript"></script>
</head>

<body>
  <div id="wrapper">
    <div id="weatherbox">
      <p><span class="weather-place"></span></p>
      <p>
        <i class="" id="weather-icon" style="font-size: 60px;"></i><span class="weather-temperature"></span>
        <br />
        <span class="weather-description" class="capitalize"></span>
        <div class="weather-sub">
          <i class="" id="weather-wind-speed"></i> | <i class="wi wi-humidity"></i> <span
            class="weather-humidity"></span> | <span class="min-max-temp"><i
              class="fas fa-long-arrow-alt-down"></i><span class="weather-min-temperature"></span> <i
              class="fas fa-long-arrow-alt-up"></i><span class="weather-max-temperature"></span></span>
        </div>
      </p>
      <p class="weather-sub"><i class="wi wi-sunrise"></i> <span class="weather-sunrise"></span> | <i
          class="wi wi-sunset"></i> <span class="weather-sunset"></span></p>
      <div id="forecast"></div>
    </div>
    <div id="spotify">
      <h1>SPOTIFY</h1>
      <img src="" id="spot-img" />
      <p id="spot-title"></p>
      <p id="spot-artist"></p>
    </div>
    <div id="clockbox">
      <div id="date">Loading...</div>
      <div id="clock">
        <div id="clockhours" class="clockhours">loading...</div>
        <div id="clockseconds" class="clockseconds">Loading...</div>
      </div>
    </div>
    <div id="calendar">
      <div id="gradient"></div>
    </div>
    <div id="complimentsbox">
      <span class="compliment">Hello!</span>
      <span class="compliment">Looking good!</span>
      <span class="compliment">Hi there!</span>
      <span class="compliment">Enjoy your day!</span>
    </div>
    <div id="rssbox">
      <div id="rsstitle">Google News</div>
      <br />
      <div id="rss-feeds"></div>
    </div>
  </div>

  <!-- Fade In Mirror -->
  <script type="text/javascript">
    $(document).ready(function () {
      $('#wrapper').fadeIn();
    });
  </script>

  <!-- Clock Module -->
  <script type="text/javascript">
    function displayTime() {
      var hours = moment().format('HH:mm');
      var seconds = moment().format('ss');
      var date = moment().format('dddd, MMMM Do YYYY');
      $('#clockhours').html(hours);
      $('#clockseconds').html(seconds);
      $('#date').html(date);
      setTimeout(displayTime, 1000);
    }
    $(document).ready(function () {
      displayTime();
    });
  </script>

  <!-- RSS Module -->
  <script type="text/javascript">
    $("#rss-feeds").rss("https://news.google.com/news/rss", {
      layoutTemplate: "<div id='newsfeed'>{entries}</div>",
      entryTemplate: "<div class='newstitle'>{title}</div>",
      ssl: true,
      limit: 10,
      dateFormat: 'DD/MM - HH:mm  '
    });

    setTimeout(function () {
      simpleslider.getSlider({
        container: document.getElementById('newsfeed'),
        duration: 1,
        delay: 15,
        prop: 'opacity',
        unit: '',
        init: 0,
        show: 1,
        end: 0
      });
    }, 20000);
  </script>

  <!-- Compliments Box Module -->
  <script type="text/javascript">
    simpleslider.getSlider({
      container: document.getElementById('complimentsbox'),
      duration: 1,
      delay: 35,
      prop: 'opacity',
      unit: '',
      init: 0,
      show: 1,
      end: 0
    });
  </script>

  <!-- Open Weather Module -->
  <script type="text/javascript">
    var owm_key = config.OWM_KEY;

    $('.weather-temperature').openWeather({
      key: owm_key,
      city: 'Kirkcaldy%2C%20GB',
      units: 'c',
      descriptionTarget: '.weather-description',
      windSpeedTarget: '#weather-wind-speed',
      minTemperatureTarget: '.weather-min-temperature',
      maxTemperatureTarget: '.weather-max-temperature',
      humidityTarget: '.weather-humidity',
      sunriseTarget: '.weather-sunrise',
      sunsetTarget: '.weather-sunset',
      placeTarget: '.weather-place',
      iconTarget: '#weather-icon',
      success: function () {
        $('.weatherbox').show();
      },
      error: function () {
        console.log("ERROR: Weather was unable to load.");
      }
    });
  </script>

  <!-- Calendar Module -->
  <script type="text/javascript">
    var gcal_key = config.GCAL_API_KEY;
    var gcal_id = config.GCAL_ID;

    $('#calendar').google_calendar_events({
      key: gcal_key, // Google Calendar API Key see: https://console.developers.google.com
      calendar: gcal_id,
      max: 5
    });
  </script>

  <!-- Spotify -->
  <script type="text/javascript">
    function spotifyRefresh() {
      $('#spotify').startSpotify({
        clientID: config.SpotClientID,
        albumImgTarget: '#spot-img',
        songTitleTarget: '#spot-title',
        songArtistTarget: '#spot-artist',
        success: function () {
          $('#spotify').show();
        },
        error: function () {
          console.log("ERROR: Spotify was unable to load.");
          $('#spotify').hide();
        }
      });
      setTimeout(spotifyRefresh, 10000);
    }
    $(document).ready(function () {
      spotifyRefresh();
    });
  </script>
</body>

</html>