"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expressioned_1 = require("./expressioned");
var UnaryExpression_1 = require("./UnaryExpression");
exports.VoidExpressionBase = expressioned_1.UnaryExpressionedNode(UnaryExpression_1.UnaryExpression);
var VoidExpression = /** @class */ (function (_super) {
    tslib_1.__extends(VoidExpression, _super);
    function VoidExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return VoidExpression;
}(exports.VoidExpressionBase));
exports.VoidExpression = VoidExpression;
