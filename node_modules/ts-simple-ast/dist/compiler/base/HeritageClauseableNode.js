"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var utils_1 = require("../../utils");
function HeritageClauseableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getHeritageClauses = function () {
            var _this = this;
            var heritageClauses = this.compilerNode.heritageClauses;
            if (heritageClauses == null)
                return [];
            return heritageClauses.map(function (c) { return _this.getNodeFromCompilerNode(c); });
        };
        class_1.prototype.getHeritageClauseByKindOrThrow = function (kind) {
            return errors.throwIfNullOrUndefined(this.getHeritageClauseByKind(kind), "Expected to have heritage clause of kind " + utils_1.getSyntaxKindName(kind) + ".");
        };
        class_1.prototype.getHeritageClauseByKind = function (kind) {
            return utils_1.ArrayUtils.find(this.getHeritageClauses(), function (c) { return c.compilerNode.token === kind; });
        };
        return class_1;
    }(Base));
}
exports.HeritageClauseableNode = HeritageClauseableNode;
