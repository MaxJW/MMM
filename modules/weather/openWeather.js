/*!
Name: Open Weather
Dependencies: jQuery, OpenWeatherMap API
Author: Michael Lynch
Author URL: http://michaelynch.com
Date Created: August 28, 2013
Licensed under the MIT license
*/
;
(function ($) {
	$.fn.openWeather = function (options) {
		// return if no element was bound
		// so chained events can continue
		if (!this.length) {
			return this;
		}
		// define default parameters
		var defaults = {
			descriptionTarget: null,
			maxTemperatureTarget: null,
			minTemperatureTarget: null,
			windSpeedTarget: null,
			humidityTarget: null,
			sunriseTarget: null,
			sunsetTarget: null,
			placeTarget: null,
			iconTarget: null,
			units: 'c',
			city: null,
			lat: null,
			lng: null,
			key: null,
			lang: 'en',
			success: function () {},
			error: function (message) {}
		}
		// define plugin
		var plugin = this;
		// define element
		var el = $(this);
		// api URL
		var apiURL;
		// define settings
		plugin.settings = {}
		// merge defaults and options
		plugin.settings = $.extend({}, defaults, options);
		// define settings namespace
		var s = plugin.settings;
		// define basic api endpoint
		apiURL = '//api.openweathermap.org/data/2.5/weather?lang=' + s.lang;
		// define API url using city (and remove any spaces in city)
		apiURL += '&q=' + s.city.replace(' ', '');
		// append api key paramater
		apiURL += '&appid=' + s.key;

		// Format Time for Sunrise/Sunset
		var formatTime = function (unixTimestamp) {
			// define milliseconds using unix time stamp
			var milliseconds = unixTimestamp * 1000;
			// create a new date using milliseconds
			var date = new Date(milliseconds);
			// define hours
			var hours = date.getHours();
			// if hours are greater than 12
			if (hours > 12) {
				// calculate remaining hours in the day
				hoursRemaining = 24 - hours;
				// define hours as the reamining hours subtracted from a 12 hour day
				hours = 12 - hoursRemaining;
			}
			// define minutes
			var minutes = date.getMinutes();
			// convert minutes to a string
			minutes = minutes.toString();
			// if minutes has less than 2 characters
			if (minutes.length < 2) {
				// add a 0 to minutes
				minutes = 0 + minutes;
			}
			// construct time using hours and minutes
			var time = hours + ':' + minutes;
			return time;
		}

		$.ajax({
			type: 'GET',
			url: apiURL,
			dataType: 'jsonp',
			success: function (data) {
				// if units are 'f'
				if (s.units == 'f') {
					// define temperature as fahrenheit
					var temperature = Math.round(((data.main.temp - 273.15) * 1.8) + 32) + '°F';
					// define min temperature as fahrenheit
					var minTemperature = Math.round(((data.main.temp_min - 273.15) * 1.8) + 32) + '°';
					// define max temperature as fahrenheit
					var maxTemperature = Math.round(((data.main.temp_max - 273.15) * 1.8) + 32) + '°';
				} else {
					// define temperature as celsius
					var temperature = ' ' + Math.round(data.main.temp - 273.15) + '°C';
					// define min temperature as celsius
					var minTemperature = Math.round(data.main.temp_min - 273.15) + '°';
					// define max temperature as celsius
					var maxTemperature = Math.round(data.main.temp_max - 273.15) + '°';
				}
				// set temperature
				el.html(temperature);
				// if minTemperatureTarget isn't null
				if (s.minTemperatureTarget != null) {
					// set minimum temperature
					$(s.minTemperatureTarget).text(minTemperature);
				}
				// if maxTemperatureTarget isn't null
				if (s.maxTemperatureTarget != null) {
					// set maximum temperature
					$(s.maxTemperatureTarget).text(maxTemperature);
				}
				// set weather description
				$(s.descriptionTarget).text(data.weather[0].description);
				// if iconTarget and default weather icon aren't null
				if (s.iconTarget != null && data.weather[0].id != null) {
					var iconID = data.weather[0].id;
					var iconURL = 'wi wi-owm-' + iconID;
					// set iconTarget src attribute as iconURL
					$(s.iconTarget).attr('class', iconURL);
				}
				// if placeTarget isn't null
				if (s.placeTarget != null) {
					// set humidity
					$(s.placeTarget).text(data.name + ', ' + data.sys.country);
				}
				// if windSpeedTarget isn't null
				if (s.windSpeedTarget != null) {
					// set wind speed
					var windSpeedNum = Math.round(data.wind.speed) + '';
					var windSpeedIcon = 'wi wi-wind-beaufort-' + windSpeedNum;
					$(s.windSpeedTarget).attr('class', windSpeedIcon)
				}
				// if humidityTarget isn't null
				if (s.humidityTarget != null) {
					// set humidity
					$(s.humidityTarget).text(data.main.humidity + '%');
				}
				// if sunriseTarget isn't null
				if (s.sunriseTarget != null) {
					var sunrise = formatTime(data.sys.sunrise);
					// set humidity
					$(s.sunriseTarget).text(sunrise + ' AM');
				}
				// if sunriseTarget isn't null
				if (s.sunsetTarget != null) {
					var sunset = formatTime(data.sys.sunset);
					// set humidity
					$(s.sunsetTarget).text(sunset + ' PM');
				}
				// run success callback
				s.success.call(this);
			},
			error: function (jqXHR, textStatus, errorThrown) {
				// run error callback
				s.error.call(this, textStatus);
			}
		});
	}
})(jQuery);