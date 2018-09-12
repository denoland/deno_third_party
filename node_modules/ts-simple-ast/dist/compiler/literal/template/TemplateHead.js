"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../../base");
var common_1 = require("../../common");
exports.TemplateHeadBase = base_1.LiteralLikeNode(common_1.Node);
var TemplateHead = /** @class */ (function (_super) {
    tslib_1.__extends(TemplateHead, _super);
    function TemplateHead() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return TemplateHead;
}(exports.TemplateHeadBase));
exports.TemplateHead = TemplateHead;
