"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var ObjectUtils = /** @class */ (function () {
    function ObjectUtils() {
    }
    ObjectUtils.clone = function (obj) {
        // todo: make this an actual deep clone... good enough for now...
        if (obj == null)
            return undefined;
        if (obj instanceof Array)
            return cloneArray(obj);
        return ObjectUtils.assign({}, obj);
        function cloneArray(a) {
            return a.map(function (item) { return ObjectUtils.clone(item); });
        }
    };
    ObjectUtils.assign = function (a, b, c) {
        if (Object.assign != null) {
            if (c == null)
                return Object.assign(a, b);
            else
                return Object.assign(a, b, c);
        }
        if (c == null)
            return this.es5Assign(a, b);
        else
            return this.es5Assign(a, b, c);
    };
    ObjectUtils.es5Assign = function (a, b, c) {
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Object/assign#Polyfill
        var to = Object(a);
        for (var index = 1; index < arguments.length; index++) {
            var nextSource = arguments[index];
            if (nextSource == null)
                continue;
            for (var nextKey in nextSource) {
                // Avoid bugs when hasOwnProperty is shadowed
                if (Object.prototype.hasOwnProperty.call(nextSource, nextKey))
                    to[nextKey] = nextSource[nextKey];
            }
        }
        return to;
    };
    return ObjectUtils;
}());
exports.ObjectUtils = ObjectUtils;
