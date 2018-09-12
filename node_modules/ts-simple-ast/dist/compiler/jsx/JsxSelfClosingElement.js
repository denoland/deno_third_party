"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var expression_1 = require("../expression");
var base_1 = require("./base");
exports.JsxSelfClosingElementBase = base_1.JsxAttributedNode(base_1.JsxTagNamedNode(expression_1.PrimaryExpression));
var JsxSelfClosingElement = /** @class */ (function (_super) {
    tslib_1.__extends(JsxSelfClosingElement, _super);
    function JsxSelfClosingElement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return JsxSelfClosingElement;
}(exports.JsxSelfClosingElementBase));
exports.JsxSelfClosingElement = JsxSelfClosingElement;
