(function ($) {
    'use strict';

    var apiEndpoint = 'https://api.spotify.com/v1/me/player';
    getSpotifyData = function() {
        var blah;

        let options = {
            url: apiEndpoint,
            headers: { 'Authorization': 'Bearer ' + this.credentials.accessToken },
            json: true
        };

        request.get(options, function(request) {blah.build(request.items)});
    }

    build = function() {
        
    }
})(jQuery, window, document);