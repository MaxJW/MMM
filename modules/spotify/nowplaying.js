(function ($) {
    $.fn.startSpotify = function (options) {
        if (!this.length) {
            return this;
        }
        var defaults = {
            authorizeEndpoint: '//accounts.spotify.com/authorize',
            tokenRefreshEndpoint: '//accounts.spotify.com/api/token',
            apiEndpoint: '//api.spotify.com/v1/me/player',
            redirectURL: 'http://192.168.64.2/index.php',
            albumImgTarget: null,
            songTitleTarget: null,
            songArtistTarget: null,
            authToken: null,
            accessToken: null,
            refreshToken: null,
            clientID: null,
            clientSecret: null,
            success: function () { },
            error: function (message) { }
        }
        var plugin = this;
        var el = $(this);
        var apiURL;
        plugin.settings = {}
        plugin.settings = $.extend({}, defaults, options);
        var s = plugin.settings;

        var getAuthorizationToken = function (FORCE_UPDATE) {
            var client_id = s.clientID;

            var scopes = 'user-read-playback-state';
            var authorizeURL = s.authorizeEndpoint + '?response_type=code' + '&client_id=' + client_id + '&scope=' + encodeURIComponent(scopes) + '&redirect_uri=' + encodeURIComponent(s.redirectURL);
            if (getUrlParameter('code') == "" || FORCE_UPDATE) {
                window.location.replace(authorizeURL);
            }

            var data = getUrlParameter('code');
            return data;
        }

        var getRefreshToken = function () {
            var client_id = s.clientID;
            var client_secret = s.clientSecret;
            var wow_data;
            $.ajax({
                method: "POST",
                url: s.tokenRefreshEndpoint,
                data: {
                    "grant_type": "authorization_code",
                    "code": s.authToken,
                    "redirect_uri": s.redirectURL,
                    "client_secret": client_secret,
                    "client_id": client_id,
                },
                headers: {
                    'Authorisation': 'Basic ' + btoa(client_id + ':' + client_secret)
                },
                async: false,
                json: true,
                success: function (result) {
                    wow_data = result;
                },
                error: function () {
                    s.authToken = null;
                    s.authToken = getAuthorizationToken(true);
                },
            });
            return wow_data;
        }

        var retrieveCurrentlyPlaying = function () {
            if (s.accessToken !== null && s.refreshToken !== null && s.authToken !== null) {
                return getSpotifyData();
            } else {
                s.authToken = getAuthorizationToken(false);
                var refreshResponse = getRefreshToken();
                s.accessToken = refreshResponse.access_token;
                s.refreshToken = refreshResponse.refresh_token;
                return getSpotifyData();
            }
        }

        var getSpotifyData = function () {
            var spotData;
            $.ajax({
                url: s.apiEndpoint,
                type: "GET",
                headers: {
                    'Authorization': 'Bearer ' + s.accessToken
                },
                async: false,
                json: true,
                success: (data) => {
                    spotData = data;
                }
            });
            return spotData;
        }

        var retrieveSongInfo = function (songInfo) {
            var sInfo = songInfo;
            if (!songInfo.noSong) {
                sInfo = {
                    imgURL: songInfo.item.album.images[1].url,
                    songTitle: songInfo.item.name,
                    artist: songInfo.item.artists[0].name,
                    album: songInfo.item.album.name,
                    titleLength: songInfo.item.duration_ms,
                    progress: songInfo.progress_ms,
                    isPlaying: songInfo.is_playing,
                    deviceName: songInfo.device.name
                };
            }
            return sInfo;
        }

        var getUrlParameter = function (name) {
            name = name.replace(/[\[]/, '\\[').replace(/[\]]/, '\\]');
            var regex = new RegExp('[\\?&]' + name + '=([^&#]*)');
            var results = regex.exec(location.search);
            return results === null ? '' : decodeURIComponent(results[1].replace(/\+/g, ' '));
        };

        var setPlayingElements = function (songInfo) {
            $(s.albumImgTarget).attr('src', songInfo.imgURL);
            $(s.songTitleTarget).text(songInfo.songTitle);
            $(s.songArtistTarget).text(songInfo.artist);
        }

        var response = retrieveCurrentlyPlaying();
        if (response) {
            var currentSongInfo = retrieveSongInfo(response);
            console.log(currentSongInfo);
            if (currentSongInfo) {
                setPlayingElements(currentSongInfo);
                s.success.call(this);
            }
        } else {
            retrieveSongInfo({
                noSong: true
            });
        }

    }
})(jQuery);

/*(function($) {
    $.fn.openWeather = function(options) {
        if (!this.length) {
            return this;
        }
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
            success: function() {},
            error: function(message) {}
        }
        var plugin = this;
        var el = $(this);
        var apiURL;
        plugin.settings = {}
        plugin.settings = $.extend({}, defaults, options);
        var s = plugin.settings;
        apiURL = '//api.openweathermap.org/data/2.5/weather?lang=' + s.lang;
        apiURL += '&q=' + s.city.replace(' ', '');
        apiURL += '&appid=' + s.key;
        var formatTime = function(unixTimestamp) {
            var milliseconds = unixTimestamp * 1000;
            var date = new Date(milliseconds);
            var hours = date.getHours();
            if (hours > 12) {
                hoursRemaining = 24 - hours;
                hours = 12 - hoursRemaining
            }
            var minutes = date.getMinutes();
            minutes = minutes.toString();
            if (minutes.length < 2) {
                minutes = 0 + minutes
            }
            var time = hours + ':' + minutes;
            return time
        }
        $.ajax({
            type: 'GET',
            url: apiURL,
            dataType: 'jsonp',
            success: function(data) {
                if (s.units == 'f') {
                    var temperature = Math.round(((data.main.temp - 273.15) * 1.8) + 32) + '°F';
                    var minTemperature = Math.round(((data.main.temp_min - 273.15) * 1.8) + 32) + '°';
                    var maxTemperature = Math.round(((data.main.temp_max - 273.15) * 1.8) + 32) + '°'
                } else {
                    var temperature = ' ' + Math.round(data.main.temp - 273.15) + '°C';
                    var minTemperature = Math.round(data.main.temp_min - 273.15) + '°';
                    var maxTemperature = Math.round(data.main.temp_max - 273.15) + '°'
                }
                el.html(temperature);
                if (s.minTemperatureTarget != null) {
                    $(s.minTemperatureTarget).text(minTemperature)
                }
                if (s.maxTemperatureTarget != null) {
                    $(s.maxTemperatureTarget).text(maxTemperature)
                }
                $(s.descriptionTarget).text(data.weather[0].description);
                if (s.iconTarget != null && data.weather[0].id != null) {
                    var iconID = data.weather[0].id;
                    var iconURL = 'wi wi-owm-' + iconID;
                    $(s.iconTarget).attr('class', iconURL)
                }
                if (s.placeTarget != null) {
                    $(s.placeTarget).text(data.name + ', ' + data.sys.country)
                }
                if (s.windSpeedTarget != null) {
                    var windSpeedNum = Math.round(data.wind.speed) + '';
                    var windSpeedIcon = 'wi wi-wind-beaufort-' + windSpeedNum;
                    $(s.windSpeedTarget).attr('class', windSpeedIcon)
                }
                if (s.humidityTarget != null) {
                    $(s.humidityTarget).text(data.main.humidity + '%')
                }
                if (s.sunriseTarget != null) {
                    var sunrise = formatTime(data.sys.sunrise);
                    $(s.sunriseTarget).text(sunrise + ' AM')
                }
                if (s.sunsetTarget != null) {
                    var sunset = formatTime(data.sys.sunset);
                    $(s.sunsetTarget).text(sunset + ' PM')
                }
                s.success.call(this)
            },
            error: function(jqXHR, textStatus, errorThrown) {
                s.error.call(this, textStatus)
            }
        })
    }
})(jQuery)*/