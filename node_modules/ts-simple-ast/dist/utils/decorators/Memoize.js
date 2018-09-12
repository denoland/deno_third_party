"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function Memoize(target, propertyName, descriptor) {
    if (descriptor.value != null)
        descriptor.value = getNewFunction(descriptor.value);
    else if (descriptor.get != null)
        descriptor.get = getNewFunction(descriptor.get);
    else
        throw new Error("Only put a Memoize decorator on a method or get accessor.");
}
exports.Memoize = Memoize;
var counter = 0;
function getNewFunction(originalFunction) {
    var identifier = ++counter;
    function decorator() {
        var propName = "__memoized_value_" + identifier;
        if (arguments.length > 0)
            propName += "_" + JSON.stringify(arguments);
        var returnedValue;
        if (this.hasOwnProperty(propName))
            returnedValue = this[propName];
        else {
            returnedValue = originalFunction.apply(this, arguments);
            Object.defineProperty(this, propName, {
                configurable: false,
                enumerable: false,
                writable: false,
                value: returnedValue
            });
        }
        return returnedValue;
    }
    return decorator;
}
