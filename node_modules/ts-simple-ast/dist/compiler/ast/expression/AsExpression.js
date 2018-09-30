"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var Expression_1 = require("./Expression");
var expressioned_1 = require("./expressioned");
exports.AsExpressionBase = base_1.TypedNode(expressioned_1.ExpressionedNode(Expression_1.Expression));
var AsExpression = /** @class */ (function (_super) {
    tslib_1.__extends(AsExpression, _super);
    function AsExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return AsExpression;
}(exports.AsExpressionBase));
exports.AsExpression = AsExpression;
