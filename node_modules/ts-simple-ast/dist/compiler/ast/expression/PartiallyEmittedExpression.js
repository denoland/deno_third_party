"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Expression_1 = require("./Expression");
var expressioned_1 = require("./expressioned");
exports.PartiallyEmittedExpressionBase = expressioned_1.ExpressionedNode(Expression_1.Expression);
var PartiallyEmittedExpression = /** @class */ (function (_super) {
    tslib_1.__extends(PartiallyEmittedExpression, _super);
    function PartiallyEmittedExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return PartiallyEmittedExpression;
}(exports.PartiallyEmittedExpressionBase));
exports.PartiallyEmittedExpression = PartiallyEmittedExpression;
