"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("./base");
var JSDocTag_1 = require("./JSDocTag");
exports.JSDocPropertyTagBase = base_1.JSDocPropertyLikeTag(JSDocTag_1.JSDocTag);
/**
 * JS doc property tag node.
 */
var JSDocPropertyTag = /** @class */ (function (_super) {
    tslib_1.__extends(JSDocPropertyTag, _super);
    function JSDocPropertyTag() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return JSDocPropertyTag;
}(exports.JSDocPropertyTagBase));
exports.JSDocPropertyTag = JSDocPropertyTag;
