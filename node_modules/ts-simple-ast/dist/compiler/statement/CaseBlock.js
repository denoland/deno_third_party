"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var base_1 = require("../base");
var common_1 = require("../common");
exports.CaseBlockBase = base_1.TextInsertableNode(common_1.Node);
var CaseBlock = /** @class */ (function (_super) {
    tslib_1.__extends(CaseBlock, _super);
    function CaseBlock() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the clauses.
     */
    CaseBlock.prototype.getClauses = function () {
        var _this = this;
        var clauses = this.compilerNode.clauses || [];
        return clauses.map(function (s) { return _this.getNodeFromCompilerNode(s); });
    };
    /**
     * Removes the clause at the specified index.
     * @param index - Index.
     */
    CaseBlock.prototype.removeClause = function (index) {
        index = manipulation_1.verifyAndGetIndex(index, this.getClauses().length - 1);
        return this.removeClauses([index, index]);
    };
    /**
     * Removes the clauses in the specified range.
     * @param indexRange - Index range.
     */
    CaseBlock.prototype.removeClauses = function (indexRange) {
        var clauses = this.getClauses();
        errors.throwIfRangeOutOfRange(indexRange, [0, clauses.length], "indexRange");
        manipulation_1.removeClausedNodeChildren(clauses.slice(indexRange[0], indexRange[1] + 1));
        return this;
    };
    return CaseBlock;
}(exports.CaseBlockBase));
exports.CaseBlock = CaseBlock;
