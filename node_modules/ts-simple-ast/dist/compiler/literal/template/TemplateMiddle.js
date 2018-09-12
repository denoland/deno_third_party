"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../../base");
var common_1 = require("../../common");
exports.TemplateMiddleBase = base_1.LiteralLikeNode(common_1.Node);
var TemplateMiddle = /** @class */ (function (_super) {
    tslib_1.__extends(TemplateMiddle, _super);
    function TemplateMiddle() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return TemplateMiddle;
}(exports.TemplateMiddleBase));
exports.TemplateMiddle = TemplateMiddle;
