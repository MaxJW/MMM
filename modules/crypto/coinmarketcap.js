(function ($) {
    $.fn.coinMarketCap = function (options) {
        var defaults = {
            BitcoinNameTarget: null,
            BitcoinPriceTarget: null,
            BitcoinPercentChangeTarget: null,
            BitcoinIconTarget: null,
            BitcoinGraphTarget: null,

            LitecoinNameTarget: null,
            LitecoinPriceTarget: null,
            LitecoinPercentChangeTarget: null,
            LitecoinIconTarget: null,
            LitecoinGraphTarget: null,

            EthereumNameTarget: null,
            EthereumPriceTarget: null,
            EthereumPercentChangeTarget: null,
            EthereumIconTarget: null,
            EthereumGraphTarget: null,

            BATNameTarget: null,
            BATPriceTarget: null,
            BATPercentChangeTarget: null,
            BATIconTarget: null,
            BATGraphTarget: null,
            apiurl: 'https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest?convert=GBP&id=1,2,1027,1697', //Bitcoin, Litecoin, Ethereum, Basic Attention Token
            success: function () { },
            error: function (message) { }
        }
        var plugin = this;
        var el = $(this);
        plugin.settings = {}
        plugin.settings = $.extend({}, defaults, options);
        var s = plugin.settings;

        var getCoinData = function () {
            $.ajax({
                url: s.apiurl,
                headers: {
                    'X-CMC_PRO_API_KEY': '28685bb3-5de2-45a2-ac2b-d423ac755686'
                },
                success: function (response) {
                    console.log(response);
                    $(s.BitcoinNameTarget).text(response.data[1].name);
                    $(s.BitcoinPriceTarget).text(response.data[1].quote.GBP.price);
                    $(s.BitcoinPercentChangeTarget).text(response.data[1].quote.GBP.percent_change_24h);
                    $(s.BitcoinIconTarget).attr('src', 'https://s2.coinmarketcap.com/static/img/coins/64x64/1.png');
                    $(s.BitcoinGraphTarget).attr('src', 'https://s2.coinmarketcap.com/generated/sparklines/web/1d/usd/1.png');
                    $(s.LitecoinNameTarget).text(response.data[2].name);
                    $(s.LitecoinPriceTarget).text(response.data[2].quote.GBP.price);
                    $(s.LitecoinPercentChangeTarget).text(response.data[2].quote.GBP.percent_change_24h);
                    $(s.LitecoinIconTarget).attr('src', 'https://s2.coinmarketcap.com/static/img/coins/64x64/2.png');
                    $(s.LitecoinGraphTarget).attr('src', 'https://s2.coinmarketcap.com/generated/sparklines/web/1d/usd/2.png');
                    $(s.EthereumNameTarget).text(response.data[1027].name);
                    $(s.EthereumPriceTarget).text(response.data[1027].quote.GBP.price);
                    $(s.EthereumPercentChangeTarget).text(response.data[1027].quote.GBP.percent_change_24h);
                    $(s.EthereumIconTarget).attr('src', 'https://s2.coinmarketcap.com/static/img/coins/64x64/1027.png');
                    $(s.EthereumGraphTarget).attr('src', 'https://s2.coinmarketcap.com/generated/sparklines/web/1d/usd/1027.png');
                    $(s.BATNameTarget).text(response.data[1697].name);
                    $(s.BATPriceTarget).text(response.data[1697].quote.GBP.price);
                    $(s.BATPercentChangeTarget).text(response.data[1697].quote.GBP.percent_change_24h);
                    $(s.BATIconTarget).attr('src', 'https://s2.coinmarketcap.com/static/img/coins/64x64/1697.png');
                    $(s.BATGraphTarget).attr('src', 'https://s2.coinmarketcap.com/generated/sparklines/web/1d/usd/1697.png');
                },
                error: function (response) {
                    console.log("coinmarket error");
                }
            });
        }

        getCoinData();
    }
})(jQuery);