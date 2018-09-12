"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
/**
 * Event container for event subscriptions.
 */
var EventContainer = /** @class */ (function () {
    function EventContainer() {
        this.subscriptions = [];
    }
    /**
     * Subscribe to an event being fired.
     * @param subscription - Subscription.
     */
    EventContainer.prototype.subscribe = function (subscription) {
        var index = this.getIndex(subscription);
        if (index === -1)
            this.subscriptions.push(subscription);
    };
    /**
     * Unsubscribe to an event being fired.
     * @param subscription - Subscription.
     */
    EventContainer.prototype.unsubscribe = function (subscription) {
        var index = this.getIndex(subscription);
        if (index >= 0)
            this.subscriptions.splice(index, 1);
    };
    /**
     * Fire an event.
     */
    EventContainer.prototype.fire = function (arg) {
        var e_1, _a;
        try {
            for (var _b = tslib_1.__values(this.subscriptions), _c = _b.next(); !_c.done; _c = _b.next()) {
                var subscription = _c.value;
                subscription(arg);
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
            }
            finally { if (e_1) throw e_1.error; }
        }
    };
    EventContainer.prototype.getIndex = function (subscription) {
        return this.subscriptions.indexOf(subscription);
    };
    return EventContainer;
}());
exports.EventContainer = EventContainer;
