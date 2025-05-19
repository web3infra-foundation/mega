/* A polyfill for browsers that don't support ligatures. */
/* The script tag referring to this file must be placed before the ending body tag. */

/* To provide support for elements dynamically added, this script adds
   method 'icomoonLiga' to the window object. You can pass element references to this method.
*/
(function () {
    'use strict';
    function supportsProperty(p) {
        var prefixes = ['Webkit', 'Moz', 'O', 'ms'],
            i,
            div = document.createElement('div'),
            ret = p in div.style;
        if (!ret) {
            p = p.charAt(0).toUpperCase() + p.substr(1);
            for (i = 0; i < prefixes.length; i += 1) {
                ret = prefixes[i] + p in div.style;
                if (ret) {
                    break;
                }
            }
        }
        return ret;
    }
    var icons;
    if (!supportsProperty('fontFeatureSettings')) {
        icons = {
            'sunrise': '&#xe900;',
            'sun': '&#xe901;',
            'moon': '&#xe902;',
            'sun2': '&#xe903;',
            'windy': '&#xe904;',
            'wind': '&#xe905;',
            'snowflake': '&#xe906;',
            'cloudy': '&#xe907;',
            'cloud': '&#xe908;',
            'weather': '&#xe909;',
            'weather2': '&#xe90a;',
            'weather3': '&#xe90b;',
            'lines': '&#xe90c;',
            'cloud2': '&#xe90d;',
            'lightning': '&#xe90e;',
            'lightning2': '&#xe90f;',
            'rainy': '&#xe910;',
            'rainy2': '&#xe911;',
            'windy2': '&#xe912;',
            'windy3': '&#xe913;',
            'snowy': '&#xe914;',
            'snowy2': '&#xe915;',
            'snowy3': '&#xe916;',
            'weather4': '&#xe917;',
            'cloudy2': '&#xe918;',
            'cloud3': '&#xe919;',
            'lightning3': '&#xe91a;',
            'sun3': '&#xe91b;',
            'moon2': '&#xe91c;',
            'cloudy3': '&#xe91d;',
            'cloud4': '&#xe91e;',
            'cloud5': '&#xe91f;',
            'lightning4': '&#xe920;',
            'rainy3': '&#xe921;',
            'rainy4': '&#xe922;',
            'windy4': '&#xe923;',
            'windy5': '&#xe924;',
            'snowy4': '&#xe925;',
            'snowy5': '&#xe926;',
            'weather5': '&#xe927;',
            'cloudy4': '&#xe928;',
            'lightning5': '&#xe929;',
            'thermometer': '&#xe92a;',
            'compass': '&#xe92b;',
            'none': '&#xe92c;',
            'celsius': '&#xe92d;',
            'fahrenheit': '&#xe92e;',
          '0': 0
        };
        delete icons['0'];
        window.icomoonLiga = function (els) {
            var classes,
                el,
                i,
                innerHTML,
                key;
            els = els || document.getElementsByTagName('*');
            if (!els.length) {
                els = [els];
            }
            for (i = 0; ; i += 1) {
                el = els[i];
                if (!el) {
                    break;
                }
                classes = el.className;
                if (/icomoon-liga/.test(classes)) {
                    innerHTML = el.innerHTML;
                    if (innerHTML && innerHTML.length > 1) {
                        for (key in icons) {
                            if (icons.hasOwnProperty(key)) {
                                innerHTML = innerHTML.replace(new RegExp(key, 'g'), icons[key]);
                            }
                        }
                        el.innerHTML = innerHTML;
                    }
                }
            }
        };
        window.icomoonLiga();
    }
}());
