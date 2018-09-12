"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var common_1 = require("../common");
exports.CatchClauseBase = common_1.Node;
var CatchClause = /** @class */ (function (_super) {
    tslib_1.__extends(CatchClause, _super);
    function CatchClause() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this catch clause's block.
     */
    CatchClause.prototype.getBlock = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.block);
    };
    /**
     * Gets this catch clause's variable declaration or undefined if none exists.
     */
    CatchClause.prototype.getVariableDeclaration = function () {
        return this.getNodeFromCompilerNodeIfExists(this.compilerNode.variableDeclaration);
    };
    /**
     * Gets this catch clause's variable declaration or throws if none exists.
     */
    CatchClause.prototype.getVariableDeclarationOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getVariableDeclaration(), "Expected to find a variable declaration.");
    };
    return CatchClause;
}(exports.CatchClauseBase));
exports.CatchClause = CatchClause;
