<!DOCTYPE html>
<html>

<head>
  <title>Magic Mirror Dashboard - Magicmirrorcentral.com</title>
  <meta http-equiv="refresh" content="900">
  <meta http-equiv="Content-type" content="text/html; charset=utf-8" />
  <script src="//ajax.googleapis.com/ajax/libs/jquery/1.9.1/jquery.min.js"
    type="39f77f089299ede1154addba-text/javascript"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/moment.js/2.1.0/moment.min.js"
    type="39f77f089299ede1154addba-text/javascript"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/Swiper/4.0.7/js/swiper.js"
    type="39f77f089299ede1154addba-text/javascript"></script>
  <script src="https://use.fontawesome.com/1545431219.js" type="39f77f089299ede1154addba-text/javascript"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/simple-slider/1.0.0/simpleslider.min.js"
    type="39f77f089299ede1154addba-text/javascript"></script>
  <link rel="stylesheet" type="text/css" href="css/style.css" />
  <link rel="stylesheet" type="text/css" href="css/weather-icons.css" />
  <link rel="stylesheet" type="text/css" href="css/jweather.css" />
  <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/Swiper/4.0.7/css/swiper.css">
  <link href="https://fonts.googleapis.com/css?family=Roboto+Condensed|Roboto:100,400" rel="stylesheet">
  <script type="39f77f089299ede1154addba-text/javascript" src="js/jweather.js"></script>
  <style type="text/css">

  </style>
  <script src="js/rss.min.js" type="39f77f089299ede1154addba-text/javascript"></script>
  <script type="39f77f089299ede1154addba-text/javascript">
    jQuery(function ($) {
      $("#rss-feeds").rss("https://news.google.com/news/rss", {
        layoutTemplate: "<div id='newsfeed'>{entries}</div>",
        entryTemplate: "<div class='newstitle'>{title}</div>",
        ssl: true,
        limit: 10,
        dateFormat: 'DD/MM - HH:mm  '
      })
    })
  </script>
</head>

<body>
  <script type="39f77f089299ede1154addba-text/javascript">
    $(window).load(function () {
      $('#wrapper').fadeIn();
    });
  </script>
  <div id="wrapper">
    <div id="weatherbox">
      <p><span class="weather-place"></span></p>
      <p><img src="" class="weather-icon" alt="Weather Icon" /><span class="weather-temperature"></span>
        <br />
        <span class="weather-description" class="capitalize"></span>
        <div class="weather-sub">
          <i class="wi wi-strong-wind"></i> <span class="weather-wind-speed"></span> | <i class="wi wi-humidity"></i>
          <span class="weather-humidity"></span> | <span class="min-max-temp"><i class="fa fa-long-arrow-down"
              aria-hidden="true"> </i><span class="weather-min-temperature"></span> <i class="fa fa-long-arrow-up"
              aria-hidden="true"></i> <span class="weather-max-temperature"></span></span>
      </p>
    </div>
    <div id="forecast"></div>
    <script type="39f77f089299ede1154addba-text/javascript">
      $(document).ready(function () {
        var forecasttext = $("#forecast").jweather({
          location: 'Dundee%2C%20GB',
          forecast: 5,
          view: "forecast",
          units: "metric", //force imperial units
        });
      });
    </script>
  </div>
  <div id="clockbox">
    <div id="date">loading...</div>
    <div id="clock">
      <div id="clockhours" class="clockhours">loading ...</div>
      <div id="clockseconds" class="clockseconds">loading ...</div>
      <div id="ampm">loading ...</div>
    </div>
  </div>
  <div id="complimentsbox">
    <span class="compliment">Hey!</span>
    <span class="compliment">Looking good!</span>
    <span class="compliment">Hi there!</span>
    <span class="compliment">Enjoy your day!</span>
  </div>
  <div id="rssbox">
    <div id="rsstitle">Google News</div>
    <br />
    <div id="rss-feeds"></div>
  </div>
  <script type="39f77f089299ede1154addba-text/javascript">
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
    }, 2000);
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
  <script type="39f77f089299ede1154addba-text/javascript">
    function displayTime() {
      var hours = moment().format('HH:mm');
      var seconds = moment().format('ss');
      var date = moment().format('dddd, MMMM Do YYYY');
      var ampm = moment().format('  ');
      $('#clockhours').html(hours);
      $('#clockseconds').html(seconds);
      $('#date').html(date);
      $('#ampm').html(ampm);
      setTimeout(displayTime, 1000);
    }
    $(document).ready(function () {
      displayTime();
    });
  </script>
  <script type="39f77f089299ede1154addba-text/javascript">
    if (typeof jQuery == 'undefined') {
      document.write(unescape("%3Cscript src='js/lib/jquery.1.9.1.min.js' type='text/javascript'%3E%3C/script%3E"));
    }
  </script>
  <script src="modules/weather/openWeather.js" type="39f77f089299ede1154addba-text/javascript"></script>
  <script type="39f77f089299ede1154addba-text/javascript">
    $(function () {

      $('.weather-temperature').openWeather({
        key: 'd2e4d541802b524195b9b8f1544ef756',
        city: 'Dundee%2C%20GB',
        units: 'c',
        descriptionTarget: '.weather-description',
        windSpeedTarget: '.weather-wind-speed',
        minTemperatureTarget: '.weather-min-temperature',
        maxTemperatureTarget: '.weather-max-temperature',
        humidityTarget: '.weather-humidity',
        sunriseTarget: '.weather-sunrise',
        sunsetTarget: '.weather-sunset',
        placeTarget: '.weather-place',
        iconTarget: '.weather-icon',
        customIcons: 'modules/weather/img/weather/',
        success: function () {
          $('.weather-wrapper').show();
        },
        error: function () {
          console.log("These aren't the droids you're looking for.");
        }
      });

    });
  </script>
  <div id="editbutton">
    <a href="index.php?mode=edit&id=ObVEk&email=maxjw98@gmail.com" <i class="fa fa-wrench" aria-hidden="true"></a></i>
  </div>
  </div>
  <script src="https://ajax.cloudflare.com/cdn-cgi/scripts/95c75768/cloudflare-static/rocket-loader.min.js"
    data-cf-settings="39f77f089299ede1154addba-|49" defer=""></script>
</body>

</html>