"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var EnableableLogger_1 = require("./EnableableLogger");
var ConsoleLogger = /** @class */ (function (_super) {
    tslib_1.__extends(ConsoleLogger, _super);
    function ConsoleLogger() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    ConsoleLogger.prototype.logInternal = function (text) {
        console.log(text);
    };
    ConsoleLogger.prototype.warnInternal = function (text) {
        console.warn(text);
    };
    return ConsoleLogger;
}(EnableableLogger_1.EnableableLogger));
exports.ConsoleLogger = ConsoleLogger;
