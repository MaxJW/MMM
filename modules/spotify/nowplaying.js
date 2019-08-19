(function ($) {
    $.fn.startSpotify = function (options) {
        var defaults = {
            authorizeEndpoint: 'https://accounts.spotify.com/authorize',
            apiEndpoint: 'https://api.spotify.com/v1/me/player',
            redirectURI: 'https://www.maxwilson.co.uk/MMM',
            clientID: null,
            access_token: null,
            scope: 'user-read-playback-state',
            albumImgTarget: null,
            songTitleTarget: null,
            songArtistTarget: null,
            success: function () { },
            error: function (message) { }
        }
        var plugin = this;
        var el = $(this);
        plugin.settings = {}
        plugin.settings = $.extend({}, defaults, options);
        var s = plugin.settings;

        var authorize = function (FORCE_UPDATE) {
            var authorizeURL = s.authorizeEndpoint + '?client_id=' + s.clientID + '&response_type=token' + '&redirect_uri=' + encodeURIComponent(s.redirectURI) + '&scope=' + encodeURIComponent(s.scope);
            var curr_url = window.location.href;

            if (!curr_url.includes('access_token') || FORCE_UPDATE) {
                window.location.replace(authorizeURL);
            } else {
                var acc = curr_url.match(/\#(?:access_token)\=([\S\s]*?)\&/)[1];
            }
            return acc;
        }

        var getSpotifyData = function () {
            $.ajax({
                url: s.apiEndpoint,
                headers: {
                    'Authorization': 'Bearer ' + s.access_token
                },
                success: function (response) {
                    console.log(response);
                    if (response !== undefined) {
                        $(s.albumImgTarget).attr('src', response.item.album.images[1].url);
                        if ((response.item.name).length > 20) {
                            //$(s.songTitleTarget).attr('class', 'marquee');
                            $(s.songTitleTarget).text((response.item.name).substring(0, 20) + '...');
                        } else {
                            $(s.songTitleTarget).attr('class', '');
                        }
                        //$(s.songTitleTarget).text(response.item.name);
                        $(s.songArtistTarget).text(response.item.artists[0].name);
                    }
                },
                error: function (response) {
                    console.log("Spotify Token Expired: Refreshing...");
                    authorize(true);
                }
            });
        }

        //Start of executed code
        if (s.access_token == null) {
            s.access_token = authorize(false);
        }
        getSpotifyData();
    }
})(jQuery);