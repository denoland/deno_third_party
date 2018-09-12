"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var expressioned_1 = require("./expressioned");
var PrimaryExpression_1 = require("./PrimaryExpression");
exports.NewExpressionBase = base_1.TypeArgumentedNode(base_1.ArgumentedNode(expressioned_1.LeftHandSideExpressionedNode(PrimaryExpression_1.PrimaryExpression)));
var NewExpression = /** @class */ (function (_super) {
    tslib_1.__extends(NewExpression, _super);
    function NewExpression() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return NewExpression;
}(exports.NewExpressionBase));
exports.NewExpression = NewExpression;
