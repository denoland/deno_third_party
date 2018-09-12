"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expression_1 = require("../expression");
exports.NullLiteralBase = expression_1.PrimaryExpression;
var NullLiteral = /** @class */ (function (_super) {
    tslib_1.__extends(NullLiteral, _super);
    function NullLiteral() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return NullLiteral;
}(exports.NullLiteralBase));
exports.NullLiteral = NullLiteral;
