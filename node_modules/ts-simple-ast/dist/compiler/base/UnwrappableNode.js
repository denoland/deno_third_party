"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
function UnwrappableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.unwrap = function () {
            manipulation_1.unwrapNode(this);
        };
        return class_1;
    }(Base));
}
exports.UnwrappableNode = UnwrappableNode;
